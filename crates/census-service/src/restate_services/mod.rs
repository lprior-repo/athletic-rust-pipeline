//! Restate service surface: durable handlers over the same [`Store`] the batch CLI drives.
//!
//! Ten definitions, one per durability need. Restate derives the service names from the struct
//! names, so these are the wire names: **`Census`**, **`Consolidate`**, **`Report`**, **`Bests`**,
//! **`Workbook`**, **`Ingest`**, **`Sweep`**, **`JurisdictionCensus`**, **`NationalCensus`**,
//! **`BrowserSession`**. Renaming a struct is a breaking API change; add a
//! `#[handler(name = "...")]` instead.
//!
//! * `Census` — request/response over the store: `status`.
//! * `Consolidate`, `Report`, `Bests`, `Workbook` — one workflow per heavy job, each addressed by a
//!   [`run_key`] the caller chooses. The job runs as a region task on the blocking pool — started
//!   through the shell's [`Spawner`], behind a semaphore sized by `--max-concurrent`, inside
//!   `ctx.run` — so a restart replays the journal value instead of redoing a completed pass, and a
//!   shutdown drain owns the job even when the invocation that started it was cancelled.
//! * `Ingest` — a virtual object keyed by endpoint (`mshsl`, `wiha`, …). Restate serializes
//!   invocations per key, which is what makes the per-endpoint cursor and window bookkeeping safe
//!   against concurrent writers.
//! * `Sweep` — a workflow that observes the ingest objects over N windows, sleeps durably between
//!   them, and leaves early when its `stop` signal is resolved.
//! * `JurisdictionCensus` — a virtual object keyed by jurisdiction identity
//!   (`jurisdiction:<state>:<season>:<revision>`). It runs that state's census stages — team index,
//!   roster walk, consolidate — recording each in durable state as it completes, so a re-invocation
//!   resumes at the stage it still owes.
//! * `NationalCensus` — the root workflow (`national:<season>:<revision>`). It fans out one
//!   `JurisdictionCensus` call per state and folds the reports into one national report, listing
//!   failed states as rows instead of failing the run.
//! * `BrowserSession` — the one headed profile, keyed [`SESSION_KEY`]: `fetch` posts a single page
//!   request to the engine and answers with its classification. It is bound only when the
//!   deployment was started with `--browser-profile`, because one process owns the profile: a
//!   second manager would be a corruption path rather than a second lane.
//!
//! Handler bodies stay thin; the work sits in free functions that take `&Store`, so the interesting
//! behaviour is testable without a Restate runtime.

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::BrowserSettings;

use crate::spawn::Spawner;
use census_crawl::net::bridge::BrowserLane;
use census_store::Store;
use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;
use tokio::sync::Semaphore;

use crate::census::CollectOptions;
use census_store::clock::Clock;
/// The workflow key one job run is addressed by: `<job>:<part>…:<generation>`.
///
/// The key names the job instance. A resubmission with the same semantic parts and the same
/// generation attaches to the retained result rather than running the job again — this is what
/// the key buys: the durability of the job does not depend on the client staying up.
///
/// A different generation (a fresh operator-visible `--generation` or `--run-id`) produces a new
/// key even when the semantic parts are identical, so an operator can ask for a fresh run of a
/// request whose result is already available. The default generation is `"1"`, which preserves
/// the attach-on-rerun behaviour for clients that do not specify one.
///
/// It lives here, next to the workflow definitions, so the batch CLI and the harness that drives
/// the deployment cannot key the same job two different ways.
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
/// The default generation a client uses when the operator does not specify one.
///
/// The same default across all callers means a rerun of the same request attaches to the existing
/// workflow, while a new `--generation` value (or a new `--run-id`) produces a fresh key and
/// starts a new run even when the semantic request is identical.
pub const DEFAULT_GENERATION: &str = "1";

mod browser_session;
mod census;
mod ingest;
mod ingest_post;
mod jobs;
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

// ---------------------------------------------------------------- wire types

pub use wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ConsolidatedTable,
    EndpointObservation, IngestReply, IngestRequest, IngestState, JurisdictionOpen,
    JurisdictionReport, JurisdictionRequest, JurisdictionState, JurisdictionSummary,
    NationalFailure, NationalReport, NationalRequest, OpenWorkReply, OpenWorkRequest,
    RefusedSource, ReportReply, ReportRequest, SealItem, SealRef, SealReply, SealRequest,
    SourceObjectOpen, SourcePlan, StageOutcome, StatusReply, SweepReport, SweepRequest, TableCount,
    WindowRequest, WorkbookReply, WorkbookRequest,
};

// ---------------------------------------------------------------- planning

// The applicability-driven plan. Public because the planner is the layer's first consumer of
// `sources::applicability` and its dispositions are what a jurisdiction report records.
pub use plan::{
    owed, plan, plan_sources, sweepable, BrowserLaneState, Dispatch, PlannedUnit, Refusal,
    UnitDisposition,
};

// ---------------------------------------------------------------- services

// The job layer the handlers share. `JobError` stays reachable at this path because a caller
// outside the module reads it; `blocking` and `job_error` are re-exported for stage submodules
// to use so they can classify `JobError` into `HandlerError` at the boundary.
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
pub use jobs::append_observations;
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

/// Signal `Sweep::interrupt` resolves to stop a running sweep.
pub const STOP_SIGNAL: &str = "stop";
/// State key: the endpoint's whole durable state, written as one value so a partially updated
/// endpoint (cursor advanced but totals not, or the reverse) cannot exist.
const KEY_STATE: &str = "state";

// The ceilings an invocation is bounded by. They stay reachable at this path because `Ingest`, the
// sweeps and the jurisdiction object read them, and `limits` holds their explanations.
pub use limits::{
    MAX_LIMIT_PER_STATE, MAX_ROWS_PER_REQUEST, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS,
};

/// The collection options one jurisdiction's walk runs under.
///
/// A zero concurrency or an over-ceiling roster limit is terminal: the request itself is wrong, and
/// replaying it would fail identically. The remaining knobs ride through as the caller set them,
/// with the collection date defaulting to today when the request does not name one.
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
    Ok(CollectOptions {
        jurisdictions: vec![request.jurisdiction],
        limit_per_state: request.limit_per_state,
        concurrency: request.concurrency,
        // One jurisdiction is one state host, so there is nothing to interleave.
        state_concurrency: 1,
        refresh: request.refresh,
        school_year: request.season,
        observed_on: request
            .observed_on
            .clone()
            .unwrap_or_else(|| today.to_string()),
    })
}

/// The service's date, read through the object context's journal.
///
/// The clock is a trait so replay *can* be deterministic, but calling it directly is not: the date
/// lands in durable state (`ctx.set`) and in `run` results that Restate compares when an entry is
/// written again, so a replay that crosses midnight would fail the invocation with a journal
/// mismatch instead of replaying it. Journaling the read makes the date the journal's.
pub(super) async fn journaled_today(
    ctx: &ObjectContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}

/// [`journaled_today`] for workflow handlers. One body per context type because the SDK's
/// `run` is a trait method whose closure type does not survive being wrapped in a generic.
pub(super) async fn journaled_today_workflow(
    ctx: &WorkflowContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    ctx.run(move || async move {
        let clock = Arc::clone(&clock);
        Ok::<_, restate_sdk::errors::HandlerError>(clock.today())
    })
    .await
    .map_err(restate_sdk::errors::HandlerError::from)
}

/// How long an invocation on this endpoint may stay in flight without journal progress.
///
/// Every handler here runs its work on the blocking pool, and the fan-out submits far more
/// jurisdiction handlers than the pool has slots, so waiting for a slot is part of an invocation's
/// in-flight time and produces no journal entry at all. Restate's one-minute default read that wait as
/// a stalled handler: on 2026-09-24 it asked 47 of the nationwide run's 49 jurisdiction invocations to
/// suspend, and aborted each one ten minutes later while it was still queued. A census handler is
/// allowed an hour of queueing; the fetch layer's own timeouts bound any single request.
const CENSUS_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// How long Restate waits for an invocation to react after asking it to suspend.
///
/// The work a handler does is not journaled until it lands, so an abort mid-job discards it and the
/// invocation's next attempt replays it. This is the backstop for a handler that is genuinely stuck
/// rather than queued: an hour is longer than any one jurisdiction's walk, and the invocation's own
/// retry policy is what decides what happens after an abort.
const CENSUS_ABORT_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// The invocation timeouts every store-backed census service declares.
///
/// Restate reads them from the manifest this endpoint publishes, per service, and falls back to its
/// own defaults — one minute without journal progress, then a ten-minute abort — when a service
/// declares neither. `BrowserSession` is deliberately not bound with these: the lane answers one
/// request at a time and a lane that hangs is better aborted on the defaults than held for an hour,
/// while a census handler that is *queued* behind the blocking pool is doing exactly what it was
/// asked to and must not be read as stalled.
fn census_service_options() -> ServiceOptions {
    ServiceOptions::new()
        .inactivity_timeout(CENSUS_INACTIVITY_TIMEOUT)
        .abort_timeout(CENSUS_ABORT_TIMEOUT)
}

/// A store-backed census service definition, with its invocation timeouts declared.
///
/// The options belong to the definition rather than to the `bind` call: that is the SDK's own
/// placement, and it keeps the timeout list next to the service it describes.
fn census_service<D: IntoServiceDefinition>(definition: D) -> ServiceDefinition {
    definition
        .into_service_definition()
        .options(census_service_options())
}

/// Build the endpoint the HTTP server serves. Service names come from the struct names: `Census`,
/// `Consolidate`, `Report`, `Bests`, `Workbook`, `Ingest`, `Sweep`, `JurisdictionCensus`,
/// `NationalCensus`, and - when the deployment serves one - `BrowserSession`.
///
/// The `region` is the shell's spawner: every blocking job these services run is started through it,
/// so a shrunk service surface still leaves nothing running that the drain does not own.
///
/// `serves_lane` is the headed profile this endpoint serves. `None` binds no `BrowserSession`, and a
/// census client that calls it anyway reads Restate's own "service not found" rather than opening a
/// second browser: who owns the profile is a deployment decision, never a fallback.
///
/// `uses_lane` is the client the census reaches that profile through, and it exists only when the
/// deployment has a lane at all. Installing it on the census fetcher is what makes a
/// browser-transported source ordinary work: the plan asks the fetcher
/// ([`BrowserLaneState::of`]), so without it the source is refused by name instead of swept.
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
