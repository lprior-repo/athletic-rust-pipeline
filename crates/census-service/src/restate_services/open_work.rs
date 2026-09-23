//! Reading the durable run's open work from the objects that own it.
//!
//! `Census::open_work` starts no work and writes nothing, so it belongs on the read surface rather
//! than in a workflow of its own. What it does need is the journal's own view — the jurisdiction
//! objects' recorded stages and the ingest objects' accepted observations — and only a context
//! inside the service can address those objects.
//!
//! The counts are `None` when nothing answered. A read that failed is not a zero, and a caller that
//! could not take a measurement must be able to say so rather than report it as an absence of work.

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{
    owed_jurisdictions, owed_source_objects, JurisdictionStages, Revision, SourceObject,
    WorkflowIdentity,
};

use super::ingest::IngestClient;
use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    JurisdictionOpen, JurisdictionState, OpenWorkReply, OpenWorkRequest, SourceObjectOpen,
};

/// Read every jurisdiction in the run scope, and every source object the caller named.
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
    Ok(OpenWorkReply {
        season: season.short(),
        revision: revision.get(),
        jurisdiction_sweeps: measured(&jurisdictions)
            .then(|| owed_jurisdictions(&stages(&jurisdictions))),
        source_objects: (!endpoints.is_empty() && measured(&endpoints))
            .then(|| owed_source_objects(&objects(&endpoints))),
        jurisdictions,
        endpoints,
    })
}

/// Whether anything answered: one row that was read is enough to count the rest.
fn measured<T>(rows: &[T]) -> bool
where
    T: ReadRow,
{
    rows.iter().any(|row| !row.unreadable())
}

/// A row that reports whether its object answered.
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

/// The stage records to count. An object that could not be read contributes a record with no stage
/// recorded, which [`JurisdictionStages::terminal`] refuses: a read that failed is not a sweep that
/// finished.
fn stages(rows: &[JurisdictionOpen]) -> Vec<JurisdictionStages> {
    rows.iter().map(|row| row.stages).collect()
}

/// The source objects to count, under the same rule.
fn objects(rows: &[SourceObjectOpen]) -> Vec<SourceObject> {
    rows.iter()
        .map(|row| SourceObject {
            endpoint: row.endpoint.clone(),
            observations: row.observations,
            windows: row.windows,
        })
        .collect()
}

/// Every jurisdiction the run scope covers, with the stages its object records.
async fn read_jurisdictions(
    ctx: &Context<'_>,
    season: SchoolYear,
    revision: Revision,
) -> Vec<JurisdictionOpen> {
    let scope = UsJurisdiction::CENSUS_SCOPE;
    let mut rows = Vec::with_capacity(scope.len());
    for jurisdiction in scope {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
        let object = ctx.object_client::<JurisdictionCensusClient>(identity.as_str());
        let Ok(Json(state)) = object.state().call().await else {
            rows.push(JurisdictionOpen {
                jurisdiction,
                identity: identity.as_str().to_string(),
                stages: JurisdictionStages::default(),
                unreadable: true,
            });
            continue;
        };
        rows.push(JurisdictionOpen {
            jurisdiction,
            identity: identity.as_str().to_string(),
            stages: stages_of(&state),
            unreadable: false,
        });
    }
    rows
}

/// What one jurisdiction's durable state says it has done.
fn stages_of(state: &JurisdictionState) -> JurisdictionStages {
    JurisdictionStages {
        teams: state.teams.is_some(),
        rosters: state.rosters.is_some(),
        meets: state.meets.is_some(),
        owed_rosters: state
            .rosters
            .as_ref()
            .map_or(0, |progress| count(progress.blocked_skipped)),
    }
}

/// One state per ingest object key the caller named.
async fn read_source_objects(ctx: &Context<'_>, endpoints: &[String]) -> Vec<SourceObjectOpen> {
    let mut rows = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        let object = ctx.object_client::<IngestClient>(endpoint.as_str());
        let row = match object.state().call().await {
            Ok(Json(state)) => SourceObjectOpen {
                endpoint: endpoint.clone(),
                observations: state.total_observations,
                windows: count(state.windows.len()),
                unreadable: false,
            },
            _ => SourceObjectOpen {
                endpoint: endpoint.clone(),
                observations: 0,
                windows: 0,
                unreadable: true,
            },
        };
        rows.push(row);
    }
    rows
}

/// A count that cannot be represented is not a count this census may claim.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
