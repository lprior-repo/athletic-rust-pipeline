//! Restate service surface: durable handlers over the same [`Store`] the batch CLI drives.
//!
//! Nine definitions, one per durability need. Restate derives the service names from the struct
//! names, so these are the wire names: **`Census`**, **`Consolidate`**, **`Report`**, **`Bests`**,
//! **`Workbook`**, **`Ingest`**, **`Sweep`**, **`JurisdictionCensus`**, **`NationalCensus`**.
//! Renaming a struct is a breaking API change; add a `#[handler(name = "...")]` instead.
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
//!
//! Handler bodies stay thin; the work sits in free functions that take `&Store`, so the interesting
//! behaviour is testable without a Restate runtime.

use crate::spawn::Spawner;
use census_store::{Store, Table};
use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Semaphore;

use crate::census::CollectOptions;
use crate::report::Scope;
use census_store::clock::Clock;

/// The workflow key one job run is addressed by: `<job>[:<part>…]:<unix seconds>`.
///
/// The key names the job instance, and a resubmission of the same key attaches to the retained
/// result rather than running the job again. The instant is part of it rather than the date: a
/// second run on the same day is a second question — the store has moved on, and answering it with
/// the first run's retained result would be wrong. What the key buys is that a *repeat* of one
/// submission attaches, so the durability of the job does not depend on the client staying up.
///
/// It lives here, next to the workflow definitions, so the batch CLI and the harness that drives the
/// deployment cannot key the same job two different ways: a date-shaped key refuses the second run
/// of a day with `the workflow method was already invoked`, which is the opposite of what the key is
/// for.
pub fn run_key(job: &str, parts: &[&str]) -> String {
    let mut key = job.to_string();
    for part in parts {
        key.push(':');
        key.push_str(part);
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    key.push(':');
    key.push_str(&seconds.to_string());
    key
}

mod census;
mod ingest;
mod jobs;
mod jurisdiction;
mod limits;
mod national;
mod open_work;
mod plan;
mod publish;
mod support;
mod sweep;
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
    owed, plan, plan_sources, sweepable, BrowserLaneState, PlannedUnit, Refusal, UnitDisposition,
};

// ---------------------------------------------------------------- services

// The job layer the handlers share. `JobError` stays reachable at this path because a caller
// outside the module reads it; `blocking` and `job_error` are the submodules' own, so the import
// stays private to this module and its children.
pub use support::JobError;
use support::{blocking, job_error};

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

/// Resolve a requested table name. Unknown names are terminal: a retry cannot fix a typo, and
/// silently creating a table nobody scans would hide the mistake.
fn resolve_table(name: &str) -> Result<Table, TerminalError> {
    Table::from_wire(name).ok_or_else(|| {
        TerminalError::new(format!(
            "unknown table {name}; expected one of {:?}",
            Table::ALL.map(Table::file)
        ))
    })
}

/// Resolve a requested table list; an empty list means every table, in [`Table::ALL`] order.
fn resolve_tables(names: &[String]) -> Result<Vec<Table>, TerminalError> {
    let mut tables = Vec::with_capacity(names.len());
    for name in names {
        let table = resolve_table(name)?;
        if !tables.contains(&table) {
            tables.push(table);
        }
    }
    if tables.is_empty() {
        return Ok(Table::ALL.to_vec());
    }
    Ok(tables)
}

/// Resolve the scope selector used by the report, bests, and workbook surfaces.
fn resolve_scope(name: Option<&str>) -> Result<Scope, TerminalError> {
    // An omitted scope means the CLI's default scope, which is every source: the two entry points
    // must not disagree about what a default report contains.
    match name {
        None | Some("all_sources") => Ok(Scope::AllSources),
        Some("core") => Ok(Scope::Core),
        Some(other) => Err(TerminalError::new(format!(
            "unknown scope {other}; expected core or all_sources"
        ))),
    }
}

fn cohort_label(grad_year: Option<i16>) -> String {
    grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string())
}

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
    Ok(ctx
        .run(move || {
            let clock = Arc::clone(&clock);
            async move { Ok::<_, HandlerError>(clock.today()) }
        })
        .await?)
}

/// [`journaled_today`] for workflow handlers. One body per context type because the SDK's
/// `run` is a trait method whose closure type does not survive being wrapped in a generic.
pub(super) async fn journaled_today_workflow(
    ctx: &WorkflowContext<'_>,
    clock: &Arc<dyn Clock>,
) -> Result<String, HandlerError> {
    let clock = Arc::clone(clock);
    Ok(ctx
        .run(move || {
            let clock = Arc::clone(&clock);
            async move { Ok::<_, HandlerError>(clock.today()) }
        })
        .await?)
}

/// Build the endpoint the HTTP server serves. Service names come from the struct names: `Census`,
/// `Consolidate`, `Report`, `Bests`, `Workbook`, `Ingest`, `Sweep`, `JurisdictionCensus`,
/// `NationalCensus`.
///
/// The `region` is the shell's spawner: every blocking job these services run is started through it,
/// so a shrunk service surface still leaves nothing running that the drain does not own.
#[tracing::instrument(skip_all, fields(max_concurrent))]
pub fn build_endpoint(store: Arc<Store>, max_concurrent: usize, region: Arc<Spawner>) -> Endpoint {
    let clock: Arc<dyn Clock> = Arc::new(census_store::clock::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent.max(1)));
    let jobs = Jobs::new(Arc::clone(&store), load, Arc::clone(&region));
    Endpoint::builder()
        .bind(Census::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            jobs.clone(),
        ))
        .bind(Consolidate::new(jobs.clone()))
        .bind(Report::new(jobs.clone()))
        .bind(Bests::new(jobs.clone()))
        .bind(Workbook::new(jobs))
        .bind(Ingest::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        ))
        .bind(Sweep::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        ))
        .bind(JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::clone(&clock),
        ))
        .bind(NationalCensus::new(clock))
        .build()
}

#[cfg(test)]
mod retry_policy_tests;

#[cfg(test)]
mod retry_policy_transport_tests;

#[cfg(test)]
mod tests;
