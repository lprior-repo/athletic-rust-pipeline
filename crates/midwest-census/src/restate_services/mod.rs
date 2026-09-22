//! Restate service surface: durable handlers over the same [`Store`] the batch CLI drives.
//!
//! Five definitions, one per durability need. Restate derives the service names from the struct
//! names, so these are the wire names: **`Census`**, **`Ingest`**, **`Sweep`**,
//! **`JurisdictionCensus`**, **`NationalCensus`**. Renaming a struct is a breaking API change; add a
//! `#[handler(name = "...")]` instead.
//!
//! * `Census` — request/response over the store: `status`, `consolidate`, `report`, `bests`,
//!   `workbook`. Each heavy job runs as a region task on the blocking pool — started through the
//!   shell's [`Spawner`], behind a semaphore sized by `--max-concurrent`, inside `ctx.run` — so a
//!   restart replays the journal value instead of redoing a completed pass, and a shutdown drain
//!   owns the job even when the invocation that started it was cancelled.
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

use crate::outcome::Outcome;
use crate::spawn::Spawner;
use crate::store::{Store, StoreError, Table};
use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Semaphore;

use crate::census::CollectOptions;
use crate::clock::Clock;
use crate::report::{ReportError, Scope};

mod census;
mod ingest;
mod jobs;
mod jurisdiction;
mod national;
mod sweep;
mod wire;

// ---------------------------------------------------------------- wire types

pub use wire::{
    BestsReply, BestsRequest, ConsolidateReply, ConsolidateRequest, ConsolidatedTable,
    EndpointObservation, IngestReply, IngestRequest, IngestState, JurisdictionReport,
    JurisdictionRequest, JurisdictionState, JurisdictionSummary, NationalFailure, NationalReport,
    NationalRequest, ReportReply, ReportRequest, StageOutcome, StatusReply, SweepReport,
    SweepRequest, TableCount, WindowRequest, WorkbookReply, WorkbookRequest,
};

// ---------------------------------------------------------------- services

pub use census::{Census, CensusClient, CensusIngressClient};
pub use ingest::{Ingest, IngestClient, IngestIngressClient};
pub use jobs::append_observations;
pub use jurisdiction::{
    JurisdictionCensus, JurisdictionCensusClient, JurisdictionCensusIngressClient,
};
pub use national::{NationalCensus, NationalCensusClient, NationalCensusIngressClient};
pub use sweep::{Sweep, SweepClient, SweepIngressClient};

/// Signal `Sweep::interrupt` resolves to stop a running sweep.
pub const STOP_SIGNAL: &str = "stop";
/// State key: the endpoint's whole durable state, written as one value so a partially updated
/// endpoint (cursor advanced but totals not, or the reverse) cannot exist.
const KEY_STATE: &str = "state";

/// Ceiling on rows in one ingest request. Callers split larger batches: the bound keeps one
/// invocation's memory and journal entry predictable.
pub const MAX_ROWS_PER_REQUEST: usize = 50_000;
/// Ceiling on windows one sweep may observe, so the durable loop is bounded by construction.
pub const MAX_SWEEP_WINDOWS: u32 = 366;

/// Ceiling on the endpoints one sweep may observe. Each endpoint costs a durable object call, so an
/// unbounded list is an admission-control hole; a larger fleet is observed by successive sweeps.
pub const MAX_SWEEP_ENDPOINTS: usize = 256;

/// Ceiling on the rosters one jurisdiction's object walks in one run. The ceiling is admission
/// control like the sweeps': a caller may ask for fewer, and a state with more teams than this is
/// covered by successive runs rather than by one unbounded invocation.
pub const MAX_LIMIT_PER_STATE: usize = 10_000;

/// A job outcome Restate can act on: retrying a transient failure is worth it, retrying a terminal
/// one is not.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// Retry with backoff — the input may still be there next time.
    #[error("{message}")]
    Transient { message: String },
    /// Do not retry — the request itself is wrong or the job panicked.
    #[error("{message}")]
    Terminal { message: String },
}

/// Store work that failed, classified for retry.
///
/// Transient is the default and the deliberate one: a lock, a full volume, an in-flight compaction —
/// each is exactly what a journaled retry repairs, and every job here is safe to repeat. An
/// `Invariant` violation is the exception: the store's own writer maintains those, so replaying the
/// same journal value cannot restore one.
impl From<StoreError> for JobError {
    fn from(error: StoreError) -> Self {
        let message = error.to_string();
        match error {
            StoreError::Invariant { .. } => Self::Terminal { message },
            _ => Self::Transient { message },
        }
    }
}

/// Report, bests and workbook work that failed: the same rule one layer up. A store failure
/// delegates so its own classification survives, and a violated invariant is terminal here for the
/// same reason it is in the store: the report's invariants are its own, so a replay cannot restore
/// one.
impl From<ReportError> for JobError {
    fn from(error: ReportError) -> Self {
        let message = error.to_string();
        match error {
            ReportError::Store(source) => Self::from(source),
            ReportError::Invariant { .. } => Self::Terminal { message },
            _ => Self::Transient { message },
        }
    }
}

/// Map a job outcome onto Restate's terminal/retryable split. Deliberately a plain function rather
/// than a `From` impl: `HandlerError` already has a blanket `From<E: StdError>`, and letting
/// `JobError` take that path would make every terminal failure silently retryable.
fn job_error(error: JobError) -> HandlerError {
    match error {
        JobError::Transient { message } => HandlerError::from(TransientFailure { message }),
        JobError::Terminal { message } => HandlerError::from(TerminalError::new(message)),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct TransientFailure {
    message: String,
}

/// Run a blocking job as a region task, classifying the outcome for retry.
///
/// `E` is whatever the job reports: the store's and the report's typed errors convert through the
/// `From` impls above, and a job that already knows its own outcome — an input bound it refused —
/// hands back a [`JobError`] unchanged. A panicked or cancelled task is always terminal: replaying
/// the journal value that panicked would panic again.
///
/// The job goes through the region the shell handed in, so an invocation that is aborted mid-await
/// leaves the work owned (and reaped) by the region rather than running unattached.
#[tracing::instrument(skip_all)]
async fn blocking<T, E, F>(spawner: Arc<Spawner>, job: F) -> Result<T, JobError>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Into<JobError> + Send + 'static,
{
    match spawner.blocking(job).await {
        Outcome::Ok(value) => Ok(value),
        Outcome::Err(error) => Err(error.into()),
        Outcome::Panicked => Err(JobError::Terminal {
            message: "job panicked".to_string(),
        }),
        Outcome::Cancelled => Err(JobError::Terminal {
            message: "job cancelled".to_string(),
        }),
        // A region job cannot report a timeout — the abort path reports a cancellation — but the
        // lattice is total and a job that never returned is terminal either way.
        Outcome::Timeout => Err(JobError::Terminal {
            message: "job timed out".to_string(),
        }),
    }
}

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

/// Build the endpoint the HTTP server serves. Service names come from the struct names: `Census`,
/// `Ingest`, `Sweep`, `JurisdictionCensus`, `NationalCensus`.
///
/// The `region` is the shell's spawner: every blocking job these services run is started through it,
/// so a shrunk service surface still leaves nothing running that the drain does not own.
#[tracing::instrument(skip_all, fields(max_concurrent))]
pub fn build_endpoint(store: Arc<Store>, max_concurrent: usize, region: Arc<Spawner>) -> Endpoint {
    let clock: Arc<dyn Clock> = Arc::new(crate::clock::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent.max(1)));
    Endpoint::builder()
        .bind(Census::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            load,
            Arc::clone(&region),
        ))
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
            Arc::clone(&region),
        ))
        .bind(NationalCensus::new(clock))
        .build()
}

#[cfg(test)]
mod tests;
