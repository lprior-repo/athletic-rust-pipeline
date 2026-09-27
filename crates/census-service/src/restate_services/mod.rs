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
pub fn run_key(job: &str, parts: &[&str], generation: &str) -> String {
    let mut key = job.to_string();
    for part in parts {
        key.push(':');
        key.push_str(part);
    }
    key.push(':');
    key.push_str(generation);
    key
}
pub const DEFAULT_GENERATION: &str = "1";

mod browser_session;
mod census;
mod ingest;
mod ingest_post;
mod jobs;
mod journaled;
mod jurisdiction;
mod limits;
mod meets_arms;
mod national;
mod open_work;
mod plan;
mod publish;
mod resolve;
mod results_arms;
mod support;
mod sweep;
mod teams_arms;
mod wire;

pub use wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ConsolidatedTable,
    EndpointObservation, IngestReply, IngestRequest, IngestState, JurisdictionOpen,
    JurisdictionReport, JurisdictionRequest, JurisdictionState, JurisdictionSummary,
    NationalFailure, NationalReport, NationalRequest, OpenWorkReply, OpenWorkRequest,
    RefusedSource, ReportReply, ReportRequest, SealItem, SealRef, SealReply, SealRequest,
    SourceObjectOpen, SourcePlan, StageOutcome, StatusReply, SweepReport, SweepRequest, TableCount,
    WindowRequest, WorkbookReply, WorkbookRequest,
};

pub use plan::{
    owed, plan, plan_sources, sweepable, BrowserLaneState, Dispatch, PlannedUnit, Refusal,
    UnitDisposition,
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
    JurisdictionCensus, JurisdictionCensusClient, JurisdictionCensusIngressClient,
};
pub use national::{NationalCensus, NationalCensusClient, NationalCensusIngressClient};
pub use publish::{
    Bests, BestsClient, BestsIngressClient, Consolidate, ConsolidateClient,
    ConsolidateIngressClient, Jobs, Report, ReportClient, ReportIngressClient, Workbook,
    WorkbookClient, WorkbookIngressClient,
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
        observed_on: request
            .observed_on
            .clone()
            .unwrap_or_else(|| today.to_string()),
        revision,
    })
}

pub(super) use journaled::{journaled_today, journaled_today_workflow};

use limits::census_service;

#[tracing::instrument(skip_all, fields(max_concurrent))]
pub fn build_endpoint(
    store: Arc<Store>,
    max_concurrent: usize,
    region: Arc<Spawner>,
    serves_lane: Option<BrowserSettings>,
    uses_lane: Option<BrowserLane>,
) -> Endpoint {
    let clock: Arc<dyn Clock> = Arc::new(census_store::clock::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent.max(1)));
    let jobs = Jobs::new(Arc::clone(&store), load, Arc::clone(&region));
    let mut builder = Endpoint::builder()
        .bind(census_service(Census::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            jobs.clone(),
        )))
        .bind(census_service(Consolidate::new(jobs.clone())))
        .bind(census_service(Report::new(jobs.clone())))
        .bind(census_service(Bests::new(jobs.clone())))
        .bind(census_service(Workbook::new(jobs)))
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
        .bind(census_service(JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            uses_lane,
        )))
        .bind(census_service(NationalCensus::new(clock)));
    if let Some(settings) = serves_lane {
        builder = builder.bind(BrowserSession::new(settings, Arc::new(SystemClock)));
    }
    builder.build()
}

#[cfg(test)]
mod retry_policy_tests;

#[cfg(test)]
mod retry_policy_transport_tests;

#[cfg(test)]
mod tests;
