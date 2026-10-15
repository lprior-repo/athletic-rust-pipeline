use census_crawl::CollectionDisposition as Disposition;
use census_domain::model::CoachLaneStatus;
use census_domain::model::Revision;
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::WorkflowIdentity;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;

use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    JurisdictionOpen, JurisdictionState, OpenWorkReply, OpenWorkRequest, SourceObjectInventory,
    SourceObjectOpen, SourcePlan,
};
use crate::census::JurisdictionStages;

mod athleticlive;
mod diagnostics;
use diagnostics::diagnostic_row;
mod history;
mod inventory;
mod source_objects;
mod store_evidence;
#[cfg(test)]
mod tests;

pub(super) use store_evidence::{inspect_store, sample_archive_activity, StoreEvidence};
const MAX_OBJECTS: usize = 1_000_000;
const MAX_ENDPOINTS: usize = 4096;

struct ReadJurisdiction {
    row: JurisdictionOpen,
    state: JurisdictionState,
}

#[tracing::instrument(skip_all)]
pub(super) async fn measure(
    ctx: &Context<'_>,
    request: &OpenWorkRequest,
    evidence: &StoreEvidence,
) -> Result<OpenWorkReply, HandlerError> {
    let season = SchoolYear::new(request.season)
        .ok_or_else(|| TerminalError::new("invalid open-work season"))?;
    let revision = Revision(request.revision);
    if !evidence.matches(season, revision) {
        return Err(TerminalError::new("open-work evidence belongs to another run").into());
    }
    if request.source_objects.len() > MAX_ENDPOINTS {
        return Err(capacity());
    }
    let jurisdictions = read_jurisdictions(ctx, season, revision, evidence).await?;
    let keys = source_objects::keys(&request.source_objects, evidence)?;
    let mut endpoints = source_objects::read(ctx, keys).await?;
    let source_inventory = source_objects::class_counts(&endpoints)?;
    inventory::append(ctx, &jurisdictions, &mut endpoints).await?;
    evidence
        .objects
        .iter()
        .try_for_each(|row| push(&mut endpoints, row.clone()))?;
    reply(
        season,
        revision,
        jurisdictions,
        endpoints,
        evidence,
        source_inventory,
    )
}

fn reply(
    season: SchoolYear,
    revision: Revision,
    mut jurisdictions: Vec<ReadJurisdiction>,
    mut endpoints: Vec<SourceObjectOpen>,
    evidence: &StoreEvidence,
    source_inventory: SourceObjectInventory,
) -> Result<OpenWorkReply, HandlerError> {
    athleticlive::project(&mut jurisdictions, &mut endpoints)?;
    let measured = jurisdictions.iter().all(|entry| !entry.row.unreadable);
    let window = jurisdictions
        .first()
        .and_then(|entry| entry.state.history_window);
    let enumerated = measured
        && evidence.bound
        && window.is_some()
        && jurisdictions.iter().all(|entry| {
            inventory::plan_covers(entry.row.jurisdiction, &entry.state)
                && entry.state.history_window == window
        });
    let jurisdiction_sweeps = measured
        .then(|| {
            count(
                jurisdictions
                    .iter()
                    .filter(|entry| {
                        !entry.row.stages.terminal() || entry.row.athleticlive_manifest.is_some()
                    })
                    .count(),
            )
        })
        .transpose()?;
    let source_objects = enumerated
        .then(|| {
            count(
                endpoints
                    .iter()
                    .filter(|row| !row.disposition.is_complete())
                    .count(),
            )
        })
        .transpose()?;
    let silent_sources = silent(&endpoints)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(jurisdictions.len())
        .map_err(|_| capacity())?;
    rows.extend(jurisdictions.into_iter().map(diagnostic_row));
    Ok(OpenWorkReply {
        season: season.short(),
        revision: revision.get(),
        jurisdiction_sweeps,
        source_objects,
        source_object_inventory: evidence.bound.then_some(source_inventory),
        silent_sources,
        endpoints,
        jurisdictions: rows,
        archive_capture_activity: evidence.archive_capture_activity.clone(),
    })
}

fn silent(endpoints: &[SourceObjectOpen]) -> Result<Vec<String>, HandlerError> {
    endpoints
        .iter()
        .filter(|row| row.disposition.is_complete() && row.observations == 0 && row.windows > 0)
        .try_fold(Vec::new(), |mut names, row| {
            push(&mut names, row.endpoint.clone())?;
            Ok(names)
        })
        .map(|mut names| {
            names.sort();
            names.dedup();
            names
        })
}

#[tracing::instrument(skip_all)]
async fn read_jurisdictions(
    ctx: &Context<'_>,
    season: SchoolYear,
    revision: Revision,
    evidence: &StoreEvidence,
) -> Result<Vec<ReadJurisdiction>, HandlerError> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(UsJurisdiction::CENSUS_SCOPE.len())
        .map_err(|_| capacity())?;
    stream::iter(UsJurisdiction::CENSUS_SCOPE).map(Ok::<_, HandlerError>)
        .try_fold(rows, |mut rows, jurisdiction| async move {
            let identity = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
            let state = ctx.object_client::<JurisdictionCensusClient>(identity.as_str()).state().call().await;
            let (state, unreadable) = match state {
                Ok(Json(state)) if state.identity == identity.as_str() => (state, false),
                Ok(Json(state)) if state.identity.is_empty() => (JurisdictionState::default(), false),
                Ok(_) => { tracing::warn!(identity = identity.as_str(), "foreign jurisdiction state"); (JurisdictionState::default(), true) }
                Err(error) => { tracing::warn!(%error, identity = identity.as_str(), "unreadable jurisdiction state"); (JurisdictionState::default(), true) }
            };
            inventory::validate(&state)?;
            rows.push(ReadJurisdiction { row: JurisdictionOpen { jurisdiction, identity: identity.as_str().to_string(),
                stages: stages_of(&state, evidence.contacts(jurisdiction), evidence.publication, evidence.rosters(jurisdiction))?, unreadable,
                teams_stage: None, source_refusals: None,
                athleticlive_manifest: None, athleticlive_manifest_reason: None }, state });
            Ok(rows)
        }).await
}

fn stages_of(
    state: &JurisdictionState,
    contacts: Disposition,
    publication: Disposition,
    authority: (Disposition, u64),
) -> Result<JurisdictionStages, HandlerError> {
    let rosters = state.rosters.as_ref();
    let native_owed = rosters
        .map(|progress| {
            count(
                progress
                    .rosters_remaining
                    .max(progress.rosters_skipped)
                    .max(progress.blocked_skipped),
            )
        })
        .transpose()?
        .map_or(0, core::convert::identity);
    let roster_status = if rosters.is_some_and(|progress| progress.is_terminal()) {
        authority.0
    } else if authority.0.is_complete() {
        Disposition::Unknown
    } else {
        authority.0
    };
    let meets = history::status(state, history::Kind::Meets)?;
    let results = history::status(state, history::Kind::Results)?;
    Ok(JurisdictionStages {
        teams: complete(state.teams.is_completed()),
        rosters: roster_status,
        meets,
        results,
        contacts,
        publication,
        refused_sources: state
            .plan
            .as_ref()
            .map(|plan| count(plan.refused.len()))
            .transpose()?
            .map_or(0, core::convert::identity),
        owed_rosters: native_owed.max(authority.1),
        owed_results: u64::from(!results.is_complete()),
    })
}

pub(super) fn collection_is_complete(
    jurisdiction: UsJurisdiction,
    state: &JurisdictionState,
) -> Result<bool, HandlerError> {
    inventory::validate(state)?;
    Ok(inventory::plan_covers(jurisdiction, state)
        && state
            .plan
            .as_ref()
            .is_some_and(|plan| refusals_settled(jurisdiction, state, plan))
        && state.teams.is_completed()
        && state
            .rosters
            .as_ref()
            .is_some_and(crate::census::StateProgress::is_terminal)
        && history::status(state, history::Kind::Meets)?.is_complete()
        && history::status(state, history::Kind::Results)?.is_complete())
}

fn refusals_settled(
    jurisdiction: UsJurisdiction,
    state: &JurisdictionState,
    plan: &SourcePlan,
) -> bool {
    plan.refused
        .iter()
        .all(|entry| accepted_coach_refusal(jurisdiction, state, entry.slug.as_str()))
}

fn accepted_coach_refusal(
    jurisdiction: UsJurisdiction,
    state: &JurisdictionState,
    slug: &str,
) -> bool {
    let Some(recorded) = crate::restate_services::wire::coach_lanes::refusal_slug(jurisdiction)
    else {
        return false;
    };
    slug == recorded
        && state
            .coach_lane
            .as_ref()
            .is_some_and(|lane| lane.status() == CoachLaneStatus::AcceptedRefusal)
}

pub(super) fn complete(completed: bool) -> Disposition {
    if completed {
        Disposition::Complete
    } else {
        Disposition::Unknown
    }
}

fn obligation(endpoint: String, disposition: Disposition) -> SourceObjectOpen {
    SourceObjectOpen {
        endpoint,
        observations: 0,
        windows: 0,
        unreadable: false,
        disposition,
        resolution: None,
        teams_source: None,
    }
}

fn push<T>(rows: &mut Vec<T>, row: T) -> Result<(), HandlerError> {
    if rows.len() >= MAX_OBJECTS {
        return Err(capacity());
    }
    rows.try_reserve(1).map_err(|_| capacity())?;
    rows.push(row);
    Ok(())
}

fn capacity() -> HandlerError {
    TerminalError::new("open-work inventory capacity exhausted; completion remains unknown").into()
}
fn count(value: usize) -> Result<u64, HandlerError> {
    u64::try_from(value).map_err(|_| capacity())
}
