use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::BrowserSettings;

use crate::spawn::Spawner;
use census_crawl::net::bridge::BrowserLane;
use census_store::Store;
use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Semaphore;

use crate::census::CollectOptions;
use census_store::clock::Clock;
pub(crate) fn run_key(job: &str, parts: &[&str], generation: &ExportGeneration) -> String {
    let mut key = job.to_string();
    for part in parts {
        key.push(':');
        key.push_str(part);
    }
    key.push(':');
    key.push_str(generation.as_str());
    key
}
pub const DEFAULT_GENERATION: &str = "1";

pub const MAX_EXPORT_GENERATION_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportGeneration(String);

#[derive(Debug, thiserror::Error)]
pub enum ExportGenerationError {
    #[error("export generation must not be empty")]
    Empty,
    #[error("export generation is {value} bytes, longer than {MAX_EXPORT_GENERATION_LEN}")]
    TooLong { value: usize },
    #[error("export generation {value:?} carries a character outside [A-Za-z0-9._-]")]
    InvalidChar { value: String },
    #[error("export key part {value:?} must not contain ':'")]
    InvalidKeyPart { value: String },
    #[error("--generation {value:?} is live-Restate only")]
    OfflineGeneration { value: String },
}

impl ExportGeneration {
    pub fn parse(value: &str) -> Result<Self, ExportGenerationError> {
        if value.is_empty() {
            return Err(ExportGenerationError::Empty);
        }
        if value.len() > MAX_EXPORT_GENERATION_LEN {
            return Err(ExportGenerationError::TooLong { value: value.len() });
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-' || byte == b'_'
        }) {
            return Err(ExportGenerationError::InvalidChar {
                value: value.to_string(),
            });
        }
        Ok(Self(value.to_string()))
    }

    pub fn default_generation() -> Self {
        Self(DEFAULT_GENERATION.to_string())
    }

    pub fn resolve(flag: Option<&str>) -> Result<Self, ExportGenerationError> {
        match flag {
            Some(value) => Self::parse(value),
            None => Ok(Self::default_generation()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn reject_offline_generation(flag: Option<&str>) -> Result<(), ExportGenerationError> {
    match flag {
        Some(value) => Err(ExportGenerationError::OfflineGeneration {
            value: value.to_string(),
        }),
        None => Ok(()),
    }
}

fn validate_key_part(part: &str) -> Result<(), ExportGenerationError> {
    if part.is_empty() || part.contains(':') {
        return Err(ExportGenerationError::InvalidKeyPart {
            value: part.to_string(),
        });
    }
    Ok(())
}

pub fn report_export_key(
    scope: &str,
    generation: &ExportGeneration,
) -> Result<String, ExportGenerationError> {
    validate_key_part(scope)?;
    Ok(run_key("report", &[scope], generation))
}

pub fn bests_export_key(
    scope: &str,
    year: &str,
    limit: &str,
    generation: &ExportGeneration,
) -> Result<String, ExportGenerationError> {
    validate_key_part(scope)?;
    validate_key_part(year)?;
    validate_key_part(limit)?;
    Ok(run_key("bests", &[scope, year, limit], generation))
}

pub fn consolidate_export_key(generation: &ExportGeneration) -> String {
    run_key("consolidate", &[], generation)
}

pub fn workbook_request_key(
    request: &WorkbookRequest,
    generation: &ExportGeneration,
) -> Result<String, ExportGenerationError> {
    let year = request
        .grad_year
        .map_or_else(|| "all".to_string(), |year| year.to_string());
    let scope = request.scope.as_deref().map_or("all", |scope| scope);
    let limit = request
        .limit
        .map_or_else(|| "all".to_string(), |limit| limit.to_string());
    let out = request.out.as_deref().map_or(".", |out| out);
    let season = request
        .school_year
        .map_or_else(|| "unstated".to_string(), |season| season.to_string());
    for part in [year.as_str(), scope, limit.as_str(), out, season.as_str()] {
        validate_key_part(part)?;
    }
    Ok(run_key(
        "workbook",
        &[&year, scope, &limit, out, &season],
        generation,
    ))
}

mod browser_session;
mod census;
mod history_stage;
mod ingest;
mod ingest_post;
mod ingest_validation;
mod jobs;
mod journaled;
pub(crate) mod jurisdiction;
mod limits;
mod meets_arms;
mod national;
mod open_work;
mod plan;
mod publish;
mod resolve;
mod results_arms;
mod school_address_join;
mod source_selection;
mod support;
mod sweep;
mod teams_arms;
mod wire;

pub use wire::{
    BestsReply, BestsRequest, BindRunReply, BindRunRequest, CompletedTeams, ConsolidateReply,
    ConsolidateRequest, ConsolidatedTable, EndpointObservation, HistoricalProgress, HistoryWindow,
    IncompleteTeams, IngestReply, IngestRequest, IngestState, JurisdictionOpen, JurisdictionOwed,
    JurisdictionReport, JurisdictionRequest, JurisdictionState, JurisdictionSummary,
    NationalFailure, NationalReport, NationalRequest, OpenWorkReply, OpenWorkRequest,
    RefusedSource, ReportReply, ReportRequest, SchoolAddressJoinReply, SchoolAddressJoinRequest,
    SealItem, SealRef, SealReply, SealRequest, SourceObjectOpen, SourcePlan, StageOutcome,
    StatusReply, SweepReport, SweepRequest, TableCount, TeamsAttemptProgress, TeamsFailure,
    TeamsSourceFailure, TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest, TeamsStage,
    WindowRequest, WorkbookReply, WorkbookRequest,
};

pub use plan::{
    owed, plan, plan_sources, sweepable, BrowserLaneState, Dispatch, PlannedUnit, Refusal,
    RefusalKind, UnitDisposition,
};

#[allow(unused_imports)]
pub(super) use jobs::collect_error;
pub(super) use resolve::{cohort_label, resolve_scope, resolve_table, resolve_tables};
pub use support::JobError;
pub use support::{blocking, job_error};

pub use browser_session::{
    BrowserSession, BrowserSessionClient, BrowserSessionDrain, BrowserSessionIngressClient,
    BrowserSessionStatus, DrainCounts, SESSION_KEY,
};
pub use census::{Census, CensusClient, CensusIngressClient};
pub use ingest::{Ingest, IngestClient, IngestIngressClient};
pub use jobs::apply_observations;
pub use jurisdiction::{
    JurisdictionCensus, JurisdictionCensusClient, JurisdictionCensusIngressClient, TeamsSource,
    TeamsSourceClient, TeamsSourceIngressClient,
};
pub use national::{NationalCensus, NationalCensusClient, NationalCensusIngressClient};
pub use publish::{
    Bests, BestsClient, BestsIngressClient, Consolidate, ConsolidateClient,
    ConsolidateIngressClient, Jobs, Report, ReportClient, ReportIngressClient, Workbook,
    WorkbookClient, WorkbookIngressClient,
};
pub use school_address_join::{
    SchoolAddressJoin, SchoolAddressJoinClient, SchoolAddressJoinIngressClient,
};
pub use sweep::{Sweep, SweepClient, SweepIngressClient};

pub const STOP_SIGNAL: &str = "stop";
const KEY_STATE: &str = "state";

pub use limits::{
    MAX_LIMIT_PER_STATE, MAX_ROWS_PER_REQUEST, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS,
};

pub(super) fn options_for_request(
    request: &JurisdictionRequest,
    today: &str,
) -> Result<CollectOptions, HandlerError> {
    if request.concurrency == 0 {
        return Err(TerminalError::new("concurrency must be at least 1").into());
    }
    limits::validate_source_parallelism(request.source_parallelism)?;
    if let Some(limit) = request.limit_per_state {
        if limit > MAX_LIMIT_PER_STATE {
            return Err(TerminalError::new(format!(
                "limit_per_state {limit} exceeds the ceiling of {MAX_LIMIT_PER_STATE}"
            ))
            .into());
        }
    }
    let revision = std::num::NonZeroU32::new(request.revision.get())
        .ok_or_else(|| TerminalError::new("revision must be at least 1"))?;
    Ok(CollectOptions {
        jurisdictions: vec![request.jurisdiction],
        limit_per_state: request.limit_per_state,
        concurrency: request.concurrency,
        state_concurrency: 1,
        refresh: request.refresh,
        school_year: request.season,
        observed_on: match request.observed_on.clone() {
            Some(value) => value,
            None => today.to_string(),
        },
        revision,
    })
}

pub(super) use journaled::{journaled_today, journaled_today_workflow};

use limits::census_service;

pub const MAX_ENDPOINT_CONCURRENCY: usize = Semaphore::MAX_PERMITS;

#[derive(Debug, thiserror::Error)]
#[error("max-concurrent {value} is outside 1..={ceiling}")]
pub struct ConcurrencyTooLarge {
    pub value: usize,
    pub ceiling: usize,
}

pub fn validate_concurrency(max_concurrent: usize) -> Result<usize, ConcurrencyTooLarge> {
    if max_concurrent == 0 || max_concurrent > MAX_ENDPOINT_CONCURRENCY {
        return Err(ConcurrencyTooLarge {
            value: max_concurrent,
            ceiling: MAX_ENDPOINT_CONCURRENCY,
        });
    }
    Ok(max_concurrent)
}

#[tracing::instrument(skip_all, fields(max_concurrent))]
pub fn build_endpoint(
    store: Arc<Store>,
    max_concurrent: usize,
    region: Arc<Spawner>,
    serves_lane: Option<BrowserSettings>,
    uses_lane: Option<BrowserLane>,
) -> Result<Endpoint, ConcurrencyTooLarge> {
    let max_concurrent = validate_concurrency(max_concurrent)?;
    let clock: Arc<dyn Clock> = Arc::new(census_store::clock::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent));
    let jobs = Jobs::new(Arc::clone(&store), load, Arc::clone(&region));
    let jurisdiction = JurisdictionCensus::new(Arc::clone(&store), Arc::clone(&clock), uses_lane);
    let mut builder = Endpoint::builder()
        .bind(census_service(Census::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            jobs.clone(),
        )))
        .bind(census_service(Consolidate::new(jobs.clone())))
        .bind(census_service(Report::new(jobs.clone())))
        .bind(census_service(Bests::new(jobs.clone())))
        .bind(census_service(Workbook::new(jobs.clone())))
        .bind(census_service(SchoolAddressJoin::new(jobs.clone())))
        .bind(census_service(Ingest::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        )))
        .bind(census_service(Sweep::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        )))
        .bind(census_service(TeamsSource::new(jurisdiction.clone(), jobs)))
        .bind(census_service(jurisdiction))
        .bind(census_service(NationalCensus::new(clock)));
    if let Some(settings) = serves_lane {
        builder = builder.bind(BrowserSession::new(settings, Arc::new(SystemClock)));
    }
    let endpoint = builder.build();
    Ok(endpoint)
}

#[cfg(test)]
mod tests;
