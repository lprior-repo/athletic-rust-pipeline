//! Restate service surface: durable handlers over the same [`Store`] the batch CLI drives.
//!
//! Three definitions, one per durability need. Restate derives the service names from the struct
//! names, so these are the wire names: **`Census`**, **`Ingest`**, **`Sweep`**. Renaming a struct is a
//! breaking API change; add a `#[handler(name = "...")]` instead.
//!
//! * `Census` — request/response over the store: `status`, `consolidate`, `report`, `bests`,
//!   `workbook`. Each heavy job runs on `spawn_blocking` behind a semaphore sized by
//!   `--max-concurrent`, inside `ctx.run`, so a restart replays the journal value instead of redoing
//!   a completed pass.
//! * `Ingest` — a virtual object keyed by endpoint (`mshsl`, `wiha`, …). Restate serializes
//!   invocations per key, which is what makes the per-endpoint cursor and window bookkeeping safe
//!   against concurrent writers.
//! * `Sweep` — a workflow that observes the ingest objects over N windows, sleeps durably between
//!   them, and leaves early when its `stop` signal is resolved.
//!
//! Handler bodies stay thin; the work sits in free functions that take `&Store`, so the interesting
//! behaviour is testable without a Restate runtime.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::bootstrap::Clock;
use crate::report::{self, Scope};
use crate::store::{Store, Table};
use crate::{bests, workbook};

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

/// Run a blocking job off the runtime, classifying the outcome for retry.
async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, JobError> {
    match tokio::task::spawn_blocking(job).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(JobError::Transient {
            message: format!("{error:#}"),
        }),
        Err(join) if join.is_panic() => Err(JobError::Terminal {
            message: format!("job panicked: {join}"),
        }),
        Err(join) => Err(JobError::Terminal {
            message: format!("job cancelled: {join}"),
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

/// Append observations for one table. Every row must carry its canonical `id`; that is what the
/// store keys the observation by.
pub fn append_observations(store: &Store, table: Table, rows: &[Value]) -> anyhow::Result<usize> {
    if rows.len() > MAX_ROWS_PER_REQUEST {
        anyhow::bail!(
            "{} rows exceeds the per-request ceiling of {MAX_ROWS_PER_REQUEST}",
            rows.len()
        );
    }
    store.append_many(table, rows)?;
    Ok(rows.len())
}

fn consolidate_tables(store: &Store, tables: &[Table]) -> anyhow::Result<Vec<ConsolidatedTable>> {
    let mut out = Vec::with_capacity(tables.len());
    for table in tables {
        let path = store.table_path(*table);
        let consolidated = store.consolidate_table(*table, &path)?;
        // Same rule as the CLI: the count comes out of the merge that wrote the snapshot.
        let emails_withheld = (*table == Table::Coaches).then_some(consolidated.withheld);
        out.push(ConsolidatedTable {
            table: table.file().to_string(),
            rows: consolidated.rows,
            emails_withheld,
        });
    }
    Ok(out)
}

fn build_report(store: &Store, scope: Scope) -> anyhow::Result<ReportReply> {
    let census = report::build_census(store, scope)?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    Ok(ReportReply {
        scope: scope.as_str().to_string(),
        generated_on: census.generated_on.clone(),
        totals: serde_json::to_value(&census.totals)?,
        json_path: json_path.display().to_string(),
        csv_path: csv_path.display().to_string(),
    })
}

fn build_bests(store: &Store, options: &bests::Options) -> anyhow::Result<BestsReply> {
    let rows = bests::build(store, options)?;
    let cohort = cohort_label(options.grad_year);
    let (jsonl, csv_path) = bests::write(store, &rows, &cohort)?;
    Ok(BestsReply {
        cohort,
        rows: rows.len(),
        jsonl: jsonl.display().to_string(),
        csv: csv_path.display().to_string(),
    })
}

fn build_workbook(store: &Store, options: &workbook::Options) -> anyhow::Result<WorkbookReply> {
    let path = workbook::build(store, options)?;
    Ok(WorkbookReply {
        path: path.display().to_string(),
        grad_year: options.grad_year,
    })
}

fn write_sweep_report(store: &Store, report: &SweepReport, today: &str) -> anyhow::Result<PathBuf> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("sweep-{today}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(report)?)?;
    Ok(path)
}

// ---------------------------------------------------------------- wire types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCount {
    pub table: String,
    pub rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReply {
    pub tables: Vec<TableCount>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub today: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidateRequest {
    /// Tables to consolidate; empty means every table.
    #[serde(default)]
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedTable {
    pub table: String,
    pub rows: usize,
    /// Consumer mailboxes withheld from the coaches snapshot by the contact contract.
    pub emails_withheld: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidateReply {
    pub tables: Vec<ConsolidatedTable>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportRequest {
    /// `core` (default) or `all_sources`.
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportReply {
    pub scope: String,
    pub generated_on: String,
    /// Totals as the census document publishes them; the per-state breakdown stays in the JSON/CSV.
    pub totals: Value,
    pub json_path: String,
    pub csv_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BestsRequest {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestsReply {
    pub cohort: String,
    pub rows: usize,
    pub jsonl: String,
    pub csv: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkbookRequest {
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookReply {
    pub path: String,
    pub grad_year: Option<i16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestState {
    /// Object key this state belongs to.
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub total_observations: u64,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub last_appended_at: Option<String>,
    #[serde(default)]
    pub windows: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Target table, e.g. `athletes`.
    pub table: String,
    /// Canonical entity observations; each row must carry its `id`.
    pub rows: Vec<Value>,
    /// Cursor the caller claims to have consumed; stored only after the append commits.
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReply {
    pub endpoint: String,
    /// Rows accepted in this request. `u64`, not `usize`: the wire shape must not change with the
    /// host pointer width.
    pub appended: u64,
    pub total_observations: u64,
    pub cursor: Option<String>,
    pub last_appended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    /// Window label being declared complete, e.g. `2026-W38`.
    pub window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRequest {
    /// Ingest object keys to observe.
    pub endpoints: Vec<String>,
    /// How many windows to observe before completing.
    #[serde(default = "default_windows")]
    pub windows: u32,
    /// Seconds slept between windows.
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
}

fn default_windows() -> u32 {
    1
}

fn default_window_seconds() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointObservation {
    pub endpoint: String,
    pub total_observations: u64,
    pub cursor: Option<String>,
    pub completed_windows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepReport {
    pub windows_observed: u32,
    pub interrupted: bool,
    pub endpoints: Vec<EndpointObservation>,
    /// Endpoints that have never accepted an observation.
    pub stale: Vec<String>,
    pub today: String,
    /// Where the durable pass wrote this report, when it reached that step.
    pub report_path: Option<String>,
}

// ---------------------------------------------------------------- services

/// `Census`: stateless request/response over the store.
#[derive(Clone)]
pub struct Census {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    load: Arc<Semaphore>,
}

impl Census {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, load: Arc<Semaphore>) -> Self {
        Self { store, clock, load }
    }

    /// Cap concurrent heavy jobs; a closed semaphore means the service is shutting down.
    async fn permit(&self) -> Result<OwnedSemaphorePermit, TerminalError> {
        Arc::clone(&self.load)
            .acquire_owned()
            .await
            .map_err(|_| TerminalError::new("service is shutting down"))
    }
}

#[service]
impl Census {
    #[handler]
    async fn status(&self, _ctx: Context<'_>) -> Result<Json<StatusReply>, HandlerError> {
        let stats = self.store.stats()?;
        let tables = stats
            .tables
            .iter()
            .map(|(table, rows)| TableCount {
                table: (*table).to_string(),
                rows: *rows,
            })
            .collect();
        Ok(Json(StatusReply {
            tables,
            observations: stats.observations,
            bytes_on_disk: stats.bytes_on_disk,
            today: self.clock.today(),
        }))
    }

    #[handler]
    async fn consolidate(
        &self,
        ctx: Context<'_>,
        Json(request): Json<ConsolidateRequest>,
    ) -> Result<Json<ConsolidateReply>, HandlerError> {
        let tables = resolve_tables(&request.tables)?;
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || consolidate_tables(&store, &tables))
                    .await
                    .map(|tables| Json(ConsolidateReply { tables }))
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn report(
        &self,
        ctx: Context<'_>,
        Json(request): Json<ReportRequest>,
    ) -> Result<Json<ReportReply>, HandlerError> {
        let scope = resolve_scope(request.scope.as_deref())?;
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_report(&store, scope))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn bests(
        &self,
        ctx: Context<'_>,
        Json(request): Json<BestsRequest>,
    ) -> Result<Json<BestsReply>, HandlerError> {
        let options = bests::Options {
            scope: resolve_scope(request.scope.as_deref())?,
            grad_year: request.grad_year,
            limit: request.limit,
        };
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_bests(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }

    #[handler]
    async fn workbook(
        &self,
        ctx: Context<'_>,
        Json(request): Json<WorkbookRequest>,
    ) -> Result<Json<WorkbookReply>, HandlerError> {
        let options = workbook::Options {
            grad_year: request.grad_year,
            limit: request.limit,
            ..workbook::Options::default()
        };
        let store = Arc::clone(&self.store);
        let permit = self.permit().await?;
        let reply = ctx
            .run(move || async move {
                let _permit = permit;
                blocking(move || build_workbook(&store, &options))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .await?;
        Ok(reply)
    }
}

/// `Ingest`: durable per-endpoint cursor and window bookkeeping, plus the append itself.
#[derive(Clone)]
pub struct Ingest {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
}

impl Ingest {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// Read the endpoint's state. An endpoint that has never recorded anything reads as empty
    /// rather than as an error: absence is the normal first-run state, not a failure.
    async fn load_object(&self, ctx: &ObjectContext<'_>) -> Result<IngestState, HandlerError> {
        let endpoint = ctx.key().to_string();
        Ok(ctx
            .get::<Json<IngestState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| IngestState {
                endpoint,
                ..IngestState::default()
            }))
    }

    /// The same read through the read-only (shared) handler context.
    async fn load_shared(
        &self,
        ctx: &SharedObjectContext<'_>,
    ) -> Result<IngestState, HandlerError> {
        let endpoint = ctx.key().to_string();
        Ok(ctx
            .get::<Json<IngestState>>(KEY_STATE)
            .await?
            .map(|state| state.0)
            .unwrap_or_else(|| IngestState {
                endpoint,
                ..IngestState::default()
            }))
    }
}

#[object]
impl Ingest {
    #[handler]
    async fn state(&self, ctx: SharedObjectContext<'_>) -> Result<Json<IngestState>, HandlerError> {
        Ok(Json(self.load_shared(&ctx).await?))
    }

    #[handler]
    async fn record(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<IngestRequest>,
    ) -> Result<Json<IngestReply>, HandlerError> {
        let table = resolve_table(&request.table)?;
        let store = Arc::clone(&self.store);
        let rows = request.rows;
        let today = self.clock.today();
        let appended = ctx
            .run(move || async move {
                blocking(move || append_observations(&store, table, &rows))
                    .await
                    // The accepted count reaches the wire as `u64`. A host where it does not fit is
                    // a hard failure: a clamped "appended" figure would be a fabricated total.
                    .and_then(|count| {
                        u64::try_from(count).map_err(|_| JobError::Terminal {
                            message: format!("appended row count {count} does not fit u64"),
                        })
                    })
                    .map_err(job_error)
            })
            .await?;

        let mut state = self.load_object(&ctx).await?;
        state.total_observations = state.total_observations.saturating_add(appended);
        if request.cursor.is_some() {
            state.cursor = request.cursor.clone();
        }
        state.last_appended_at = Some(today);

        // One write, not four: the cursor, the totals, and the timestamp can never disagree.
        ctx.set(KEY_STATE, Json(state.clone()));

        Ok(Json(IngestReply {
            endpoint: state.endpoint,
            appended,
            total_observations: state.total_observations,
            cursor: state.cursor,
            last_appended_at: state.last_appended_at,
        }))
    }

    #[handler]
    async fn complete_window(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<WindowRequest>,
    ) -> Result<Json<IngestState>, HandlerError> {
        if request.window.trim().is_empty() {
            return Err(TerminalError::new("window label must not be empty").into());
        }
        let mut state = self.load_object(&ctx).await?;
        if !state.windows.contains(&request.window) {
            state.windows.push(request.window);
            state.windows.sort();
            ctx.set(KEY_STATE, Json(state.clone()));
        }
        Ok(Json(state))
    }
}

/// `Sweep`: observe the ingest objects across windows, durably sleeping between them.
#[derive(Clone)]
pub struct Sweep {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
}

impl Sweep {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }
}

#[workflow]
impl Sweep {
    #[handler]
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        Json(request): Json<SweepRequest>,
    ) -> Result<Json<SweepReport>, HandlerError> {
        if request.windows > MAX_SWEEP_WINDOWS {
            return Err(TerminalError::new(format!(
                "{} windows exceeds the ceiling of {MAX_SWEEP_WINDOWS}",
                request.windows
            ))
            .into());
        }
        if request.endpoints.len() > MAX_SWEEP_ENDPOINTS {
            return Err(TerminalError::new(format!(
                "{} endpoints exceeds the ceiling of {MAX_SWEEP_ENDPOINTS}",
                request.endpoints.len()
            ))
            .into());
        }
        let today = self.clock.today();
        let (windows_observed, interrupted) =
            Self::wait_windows(&ctx, request.windows, request.window_seconds).await?;
        let (endpoints, stale) = Self::observe_endpoints(&ctx, &request.endpoints).await?;

        let mut report = SweepReport {
            windows_observed,
            interrupted,
            endpoints,
            stale,
            today: today.clone(),
            report_path: None,
        };
        let store = Arc::clone(&self.store);
        let written = ctx
            .run(move || async move {
                blocking(move || {
                    let path = write_sweep_report(&store, &report, &today)?;
                    report.report_path = Some(path.display().to_string());
                    Ok(Json(report))
                })
                .await
                .map_err(job_error)
            })
            .await?;
        Ok(written)
    }

    /// Sleep out `windows` durable windows, returning how many elapsed and whether the stop signal
    /// cut the wait short. `window_seconds == 0` skips the sleep but still honors the signal.
    ///
    /// Cancellation-safe by construction: both select branches are durable Restate futures, so a
    /// drop mid-select replays the branch from its start instead of losing the wakeup.
    async fn wait_windows(
        ctx: &WorkflowContext<'_>,
        windows: u32,
        window_seconds: u64,
    ) -> Result<(u32, bool), HandlerError> {
        let mut observed = 0_u32;
        for _ in 0..windows {
            if window_seconds == 0 {
                if ctx.signal::<String>(STOP_SIGNAL).await.is_ok() {
                    return Ok((observed, true));
                }
            } else {
                let slept = tokio::select! {
                    stop = ctx.signal::<String>(STOP_SIGNAL) => {
                        // The signal payload is the wake reason, not data: reaching here is what matters.
                        let _signal = stop?;
                        false
                    }
                    outcome = ctx.sleep(Duration::from_secs(window_seconds)) => {
                        outcome?;
                        true
                    }
                };
                if !slept {
                    return Ok((observed, true));
                }
            }
            observed = observed.saturating_add(1);
        }
        Ok((observed, false))
    }

    /// Ask every endpoint for its state once, in request order, and note the ones that have written
    /// nothing yet. A bounded slice keeps the object calls bounded.
    async fn observe_endpoints(
        ctx: &WorkflowContext<'_>,
        endpoints: &[String],
    ) -> Result<(Vec<EndpointObservation>, Vec<String>), HandlerError> {
        let mut observed = Vec::with_capacity(endpoints.len());
        let mut stale = Vec::new();
        for endpoint in endpoints {
            let state = ctx
                .object_client::<IngestClient>(endpoint.clone())
                .state()
                .call()
                .await?
                .0;
            if state.total_observations == 0 {
                stale.push(endpoint.clone());
            }
            observed.push(EndpointObservation {
                endpoint: endpoint.clone(),
                total_observations: state.total_observations,
                cursor: state.cursor,
                completed_windows: state.windows.len(),
            });
        }
        Ok((observed, stale))
    }

    #[handler]
    async fn interrupt(
        &self,
        ctx: SharedWorkflowContext<'_>,
        invocation_id: String,
    ) -> Result<Json<bool>, HandlerError> {
        if invocation_id.trim().is_empty() {
            return Err(TerminalError::new("invocation id must not be empty").into());
        }
        ctx.invocation_handle(invocation_id)
            .signal(STOP_SIGNAL)
            .resolve(STOP_SIGNAL.to_string());
        Ok(Json(true))
    }
}

/// Build the endpoint the HTTP server serves. Service names come from the struct names: `Census`,
/// `Ingest`, `Sweep`.
pub fn build_endpoint(store: Arc<Store>, max_concurrent: usize) -> Endpoint {
    let clock: Arc<dyn Clock> = Arc::new(crate::bootstrap::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent.max(1)));
    Endpoint::builder()
        .bind(Census::new(Arc::clone(&store), Arc::clone(&clock), load))
        .bind(Ingest::new(Arc::clone(&store), Arc::clone(&clock)))
        .bind(Sweep::new(store, clock))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_names_resolve_and_reject_typos() {
        assert_eq!(resolve_table("schools").unwrap(), Table::Schools);
        assert_eq!(resolve_table("performances").unwrap(), Table::Performances);
        assert!(resolve_table("school").is_err());
        assert!(resolve_table("").is_err());
    }

    #[test]
    fn empty_table_list_means_every_table_in_order() {
        assert_eq!(resolve_tables(&[]).unwrap(), Table::ALL.to_vec());
        let requested = vec!["meets".to_string(), "meets".to_string()];
        assert_eq!(resolve_tables(&requested).unwrap(), vec![Table::Meets]);
    }

    #[test]
    fn an_omitted_scope_matches_the_cli_default_and_unknown_scopes_are_rejected() {
        // `midwest-census report` without `--core` reports every source, so the service must too.
        assert_eq!(resolve_scope(None).unwrap(), Scope::AllSources);
        assert_eq!(resolve_scope(Some("core")).unwrap(), Scope::Core);
        assert_eq!(
            resolve_scope(Some("all_sources")).unwrap(),
            Scope::AllSources
        );
        assert!(resolve_scope(Some("all")).is_err());
    }

    #[test]
    fn cohort_label_names_the_reduction() {
        assert_eq!(cohort_label(Some(2027)), "co2027");
        assert_eq!(cohort_label(None), "all");
    }

    #[test]
    fn rows_without_an_id_are_rejected_by_the_store_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let rows = vec![serde_json::json!({"name": "no id here"})];
        assert!(append_observations(&store, Table::Schools, &rows).is_err());
        assert_eq!(store.stats().unwrap().observations, 0);

        let good = vec![serde_json::json!({"id": "school:wi:test", "name": "Test"})];
        assert_eq!(
            append_observations(&store, Table::Schools, &good).unwrap(),
            1
        );
        assert_eq!(store.stats().unwrap().observations, 1);
    }

    #[test]
    fn oversized_batches_are_refused_without_touching_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let rows = vec![serde_json::json!({"id": "x"}); MAX_ROWS_PER_REQUEST + 1];
        assert!(append_observations(&store, Table::Schools, &rows).is_err());
        assert_eq!(store.stats().unwrap().observations, 0);
    }

    #[test]
    fn transient_jobs_are_retryable_and_panics_are_terminal() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let transient = runtime.block_on(blocking(|| -> anyhow::Result<u8> {
            anyhow::bail!("disk went away")
        }));
        assert!(matches!(transient, Err(JobError::Transient { .. })));
        let panicked = runtime.block_on(blocking(|| -> anyhow::Result<u8> {
            panic!("boom");
        }));
        assert!(matches!(panicked, Err(JobError::Terminal { .. })));
    }
}
