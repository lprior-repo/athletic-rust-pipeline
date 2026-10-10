use census_crawl::CollectionDisposition as Disposition;
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;

use super::ingest::IngestClient;
use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    JurisdictionOpen, JurisdictionState, OpenWorkReply, OpenWorkRequest, SourceObjectOpen,
};
use crate::census::JurisdictionStages;

mod history;
mod inventory;
mod store_evidence;
#[cfg(test)]
mod tests;

pub(super) use store_evidence::{inspect_store, StoreEvidence};
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
    let jurisdictions = read_jurisdictions(ctx, season, revision, evidence).await?;
    let mut endpoints = read_source_objects(ctx, &request.source_objects).await?;
    inventory::append(ctx, &jurisdictions, &mut endpoints).await?;
    evidence
        .objects
        .iter()
        .try_for_each(|row| push(&mut endpoints, row.clone()))?;
    reply(season, revision, jurisdictions, endpoints, evidence)
}

fn reply(
    season: SchoolYear,
    revision: Revision,
    jurisdictions: Vec<ReadJurisdiction>,
    endpoints: Vec<SourceObjectOpen>,
    evidence: &StoreEvidence,
) -> Result<OpenWorkReply, HandlerError> {
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
                    .filter(|entry| !entry.row.stages.terminal())
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
    rows.extend(jurisdictions.into_iter().map(|entry| entry.row));
    Ok(OpenWorkReply {
        season: season.short(),
        revision: revision.get(),
        jurisdiction_sweeps,
        source_objects,
        silent_sources,
        endpoints,
        jurisdictions: rows,
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
                stages: stages_of(&state, evidence.contacts(jurisdiction), evidence.publication, evidence.rosters(jurisdiction))?, unreadable }, state });
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
            .is_some_and(|plan| plan.refused.is_empty())
        && state.teams.is_completed()
        && state
            .rosters
            .as_ref()
            .is_some_and(crate::census::StateProgress::is_terminal)
        && history::status(state, history::Kind::Meets)?.is_complete()
        && history::status(state, history::Kind::Results)?.is_complete())
}

pub(super) fn complete(completed: bool) -> Disposition {
    if completed {
        Disposition::Complete
    } else {
        Disposition::Unknown
    }
}

#[tracing::instrument(skip_all)]
async fn read_source_objects(
    ctx: &Context<'_>,
    endpoints: &[String],
) -> Result<Vec<SourceObjectOpen>, HandlerError> {
    if endpoints.len() > MAX_ENDPOINTS
        || endpoints
            .iter()
            .any(|endpoint| endpoint.len() > MAX_ENDPOINTS)
    {
        return Err(capacity());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(endpoints.len())
        .map_err(|_| capacity())?;
    stream::iter(endpoints)
        .map(Ok::<_, HandlerError>)
        .try_fold(rows, |mut rows, endpoint| async move {
            let mut row = obligation(endpoint.clone(), Disposition::Unknown);
            match ctx
                .object_client::<IngestClient>(endpoint.as_str())
                .state()
                .call()
                .await
            {
                Ok(Json(state)) if state.endpoint == *endpoint => {
                    row.observations = state.total_observations;
                    row.windows = state.completed_windows();
                }
                Ok(_) => {
                    tracing::warn!(endpoint, "foreign ingest source object");
                    row.unreadable = true;
                }
                Err(error) => {
                    tracing::warn!(%error, endpoint, "unreadable ingest source object");
                    row.unreadable = true;
                }
            }
            rows.push(row);
            Ok(rows)
        })
        .await
}

fn obligation(endpoint: String, disposition: Disposition) -> SourceObjectOpen {
    SourceObjectOpen {
        endpoint,
        observations: 0,
        windows: 0,
        unreadable: false,
        disposition,
        resolution: None,
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
