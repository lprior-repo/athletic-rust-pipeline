use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{
    owed_jurisdictions, owed_source_objects, silent_source_objects, JurisdictionStages,
    SourceObject,
};
use census_reconcile::identity::{Revision, WorkflowIdentity};

use super::ingest::IngestClient;
use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    JurisdictionOpen, JurisdictionState, OpenWorkReply, OpenWorkRequest, SourceObjectOpen,
};

pub(super) async fn measure(
    ctx: &Context<'_>,
    request: &OpenWorkRequest,
) -> Result<OpenWorkReply, HandlerError> {
    let season = SchoolYear::new(request.season).ok_or_else(|| {
        TerminalError::new(format!(
            "season year {} is not a school year",
            request.season
        ))
    })?;
    let revision = Revision(request.revision);
    let jurisdictions = read_jurisdictions(ctx, season, revision).await;
    let endpoints = read_source_objects(ctx, &request.source_objects).await;
    let source_objects = objects(&endpoints);
    Ok(OpenWorkReply {
        season: season.short(),
        revision: revision.get(),
        jurisdiction_sweeps: measured(&jurisdictions)
            .then(|| owed_jurisdictions(&stages(&jurisdictions))),
        source_objects: (!endpoints.is_empty() && measured(&endpoints))
            .then(|| owed_source_objects(&source_objects)),
        silent_sources: silent_source_objects(&source_objects),
        jurisdictions,
        endpoints,
    })
}

fn measured<T>(rows: &[T]) -> bool
where
    T: ReadRow,
{
    rows.iter().any(|row| !row.unreadable())
}

trait ReadRow {
    fn unreadable(&self) -> bool;
}

impl ReadRow for JurisdictionOpen {
    fn unreadable(&self) -> bool {
        self.unreadable
    }
}

impl ReadRow for SourceObjectOpen {
    fn unreadable(&self) -> bool {
        self.unreadable
    }
}

fn stages(rows: &[JurisdictionOpen]) -> Vec<JurisdictionStages> {
    rows.iter().map(|row| row.stages).collect()
}

fn objects(rows: &[SourceObjectOpen]) -> Vec<SourceObject> {
    rows.iter()
        .map(|row| SourceObject {
            endpoint: row.endpoint.clone(),
            observations: row.observations,
            windows: row.windows,
        })
        .collect()
}

async fn read_jurisdictions(
    ctx: &Context<'_>,
    season: SchoolYear,
    revision: Revision,
) -> Vec<JurisdictionOpen> {
    let scope = UsJurisdiction::CENSUS_SCOPE;
    let mut rows: Vec<JurisdictionOpen> = Vec::with_capacity(scope.len());
    let mut in_flight = DurableFuturesUnordered::new();
    for jurisdiction in scope {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
        let object = ctx.object_client::<JurisdictionCensusClient>(identity.as_str());
        in_flight.push(object.state().call());
        rows.push(JurisdictionOpen {
            jurisdiction,
            identity: identity.as_str().to_string(),
            stages: JurisdictionStages::default(),
            unreadable: true,
        });
    }
    while let Ok(Some((index, outcome))) = in_flight.next().await {
        let Ok(Json(state)) = outcome else {
            continue;
        };
        if let Some(row) = rows.get_mut(index) {
            row.stages = stages_of(&state);
            row.unreadable = false;
        }
    }
    rows
}

fn stages_of(state: &JurisdictionState) -> JurisdictionStages {
    let results = state.results.as_ref();
    let results_complete = results.is_some_and(|outcome| {
        outcome.per_source.iter().all(|source| {
            source.errors == 0
                && source
                    .unresolved
                    .as_ref()
                    .is_none_or(|u| u.rows == 0 && u.labels == 0)
        })
    });
    let owed_results = if results.is_none() || !results_complete {
        1
    } else {
        0
    };
    JurisdictionStages {
        teams: state.teams.is_completed(),
        rosters: state
            .rosters
            .as_ref()
            .is_some_and(|progress| progress.is_terminal()),
        meets: state.meets_complete,
        results: results_complete,
        owed_rosters: state.rosters.as_ref().map_or(0, |progress| {
            count(progress.rosters_remaining.max(progress.blocked_skipped))
        }),
        owed_results,
    }
}

async fn read_source_objects(ctx: &Context<'_>, endpoints: &[String]) -> Vec<SourceObjectOpen> {
    let mut rows: Vec<SourceObjectOpen> = Vec::with_capacity(endpoints.len());
    let mut in_flight = DurableFuturesUnordered::new();
    for endpoint in endpoints {
        let object = ctx.object_client::<IngestClient>(endpoint.as_str());
        in_flight.push(object.state().call());
        rows.push(SourceObjectOpen {
            endpoint: endpoint.clone(),
            observations: 0,
            windows: 0,
            unreadable: true,
        });
    }
    while let Ok(Some((index, outcome))) = in_flight.next().await {
        let Ok(Json(state)) = outcome else {
            continue;
        };
        if let Some(row) = rows.get_mut(index) {
            row.observations = state.total_observations;
            row.windows = state.completed_windows();
            row.unreadable = false;
        }
    }
    rows
}

fn count(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}

#[cfg(test)]
mod tests;
