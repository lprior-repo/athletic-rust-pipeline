use restate_sdk::prelude::*;

mod capture_execution;
mod work_admission;
pub use capture_execution::CaptureExecution;

use crate::census::CollectOptions;

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

pub fn jurisdictions_key_part(jurisdictions: &[census_domain::UsJurisdiction]) -> String {
    if jurisdictions.is_empty() {
        return "all".to_string();
    }
    let mut codes: Vec<&str> = jurisdictions
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes.join(",")
}

pub fn report_export_key(
    scope: &str,
    jurisdictions: &[census_domain::UsJurisdiction],
    generation: &ExportGeneration,
) -> Result<String, ExportGenerationError> {
    let places = jurisdictions_key_part(jurisdictions);
    validate_key_part(scope)?;
    validate_key_part(&places)?;
    Ok(run_key("report", &[scope, &places], generation))
}

pub fn bests_export_key(
    scope: &str,
    year: &str,
    limit: &str,
    jurisdictions: &[census_domain::UsJurisdiction],
    generation: &ExportGeneration,
) -> Result<String, ExportGenerationError> {
    let places = jurisdictions_key_part(jurisdictions);
    validate_key_part(scope)?;
    validate_key_part(year)?;
    validate_key_part(limit)?;
    validate_key_part(&places)?;
    Ok(run_key("bests", &[scope, year, limit, &places], generation))
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
    let places = jurisdictions_key_part(&request.jurisdictions);
    let out = request.out.as_deref().map_or(".", |out| out);
    let season = request
        .school_year
        .map_or_else(|| "unstated".to_string(), |season| season.to_string());
    for part in [
        year.as_str(),
        scope,
        limit.as_str(),
        places.as_str(),
        out,
        season.as_str(),
    ] {
        validate_key_part(part)?;
    }
    Ok(run_key(
        "workbook",
        &[&year, scope, &limit, &places, out, &season],
        generation,
    ))
}

mod browser_session;
mod census;
mod endpoint;
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
    NationalCompleteness, NationalFailure, NationalReport, NationalRequest, NationalRequestBinding,
    OpenWorkReply, OpenWorkRequest, PublicationCapabilities, PublicationContract, RefusedSource,
    ReportReply, ReportRequest, SchoolAddressJoinReply, SchoolAddressJoinRequest, SealItem,
    SealRef, SealReply, SealRequest, SourceObjectInventory, SourceObjectOpen, SourcePlan,
    StageOutcome, StatusReply, SweepReport, SweepRequest, TableCount, TeamsAttemptProgress,
    TeamsFailure, TeamsSourceFailure, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest, TeamsStage, WindowRequest, WorkbookReply, WorkbookRequest,
};

pub use plan::{
    owed, plan, plan_sources, sweepable, BrowserLaneState, Dispatch, PlannedUnit, Refusal,
    RefusalKind, UnitDisposition,
};

#[allow(unused_imports)]
pub(super) use jobs::collect_error;
#[cfg(test)]
pub(super) use resolve::MAX_REQUEST_JURISDICTIONS;
pub(super) use resolve::{
    cohort_label, normalize_jurisdictions, resolve_scope, resolve_table, resolve_tables,
};
pub use support::JobError;
pub use support::{blocking, job_error};

pub use browser_session::{
    BrowserSession, BrowserSessionClient, BrowserSessionDrain, BrowserSessionIngressClient,
    BrowserSessionStatus, DrainCounts, SESSION_KEY,
};
pub use census::{Census, CensusClient, CensusIngressClient};
pub use endpoint::{
    build_endpoint, validate_concurrency, ConcurrencyTooLarge, MAX_ENDPOINT_CONCURRENCY,
};
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

#[cfg(test)]
use crate::spawn::Spawner;
#[cfg(test)]
use census_store::Store;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
mod tests;
