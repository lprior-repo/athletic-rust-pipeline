mod capture_execution;
mod work_admission;
pub use capture_execution::CaptureExecution;

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

mod export_generation;
pub use export_generation::{
    reject_offline_generation, ExportGeneration, ExportGenerationError, DEFAULT_GENERATION,
    MAX_EXPORT_GENERATION_LEN,
};

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
    let census_report::export::ExportJobRequest::SelectedPublicationV1 { selection } =
        &request.export_job;
    let digest = census_domain::model::serialized_digest(selection)
        .map_err(ExportGenerationError::SelectedRequestDigest)?;
    let selected = format!("selected_v1_{digest}");
    Ok(run_key(
        "workbook",
        &[
            year.as_str(),
            scope,
            limit.as_str(),
            places.as_str(),
            out,
            season.as_str(),
            selected.as_str(),
        ],
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
    default_pass_budget, default_pass_delay_seconds, ArchiveCaptureActivity, ArchiveCaptureEpoch,
    ArchiveCaptureSample, ArchiveCaptureUnmeasured, ArchiveUnmeasuredReason, BestsReply,
    BestsRequest, BindRunReply, BindRunRequest, CompletedTeams, ConsolidateReply,
    ConsolidateRequest, ConsolidatedTable, ContactStatus, ContactSummary, EndpointObservation,
    HistoricalProgress, HistoryWindow, IncompleteTeams, IngestReply, IngestRequest, IngestState,
    JurisdictionOpen, JurisdictionReport, JurisdictionRequest, JurisdictionState, NationalRequest,
    NationalRequestBinding, OpenWorkReply, OpenWorkRequest, OriginHeldContinuationRequest,
    PublicationCapabilities, PublicationContract, RefusedSource, ReportReply, ReportRequest,
    SchoolAddressJoinRequest, SealItem, SealRef, SealReply, SealRequest, SourceObjectInventory,
    SourceObjectOpen, SourceObjectPageReply, SourceObjectPageRequest, SourcePlan, StageOutcome,
    StatusReply, SweepReport, SweepRequest, TableCount, TeamsAttemptProgress,
    TeamsContinuationOutcome, TeamsFailure, TeamsReconciliationRequest, TeamsSourceFailure,
    TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest, TeamsStage, WindowRequest,
    WorkbookReply, WorkbookRequest, ARCHIVE_ACTIVITY_METRIC_VERSION,
};

pub use plan::{
    compute_plan_fingerprint_with_nces_receipt, owed, plan, plan_sources, sweepable,
    BrowserLaneState, Dispatch, PlannedUnit, Refusal, RefusalKind, UnitDisposition,
};

#[allow(unused_imports)]
pub(super) use jobs::collect_error;
#[cfg(test)]
pub(super) use resolve::MAX_REQUEST_JURISDICTIONS;
pub(super) use resolve::{
    cohort_label, normalize_jurisdictions, options_for_request, resolve_scope, resolve_table,
    resolve_tables,
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

pub(super) use journaled::{journaled_today, journaled_today_workflow};

#[cfg(test)]
use crate::spawn::Spawner;
#[cfg(test)]
use census_store::Store;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
mod tests;
