use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_store::clock::Clock;
use census_store::Store;
use chrono::{Days, NaiveDate};

use super::ingest::IngestClient;
use super::jobs::write_sweep_report;
use super::wire::ingest::{EndpointObservation, SweepReport, SweepRequest};
use super::{job_error, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS, STOP_SIGNAL};

mod blocking_prune_receipts;
mod blocking_write_report;

/// How long a store receipt must outlive the operation it names.
///
/// While Restate still holds an invocation's journal, that invocation can be replayed — and a replay
/// has to find the receipt the first application wrote, or it appends the page again. The objects
/// that write receipts declare a 90-day journal retention, so 90 days is the window; it is
/// deliberately the *longest* retention any of them declares, not the shortest.
///
/// This is the policy half of [`Store::prune_receipts`]. The store holds no opinion on the window;
/// it removes exactly the receipts a caller tells it are past theirs.
pub const REPLAY_RETENTION_DAYS: u64 = 90;

/// The oldest day a receipt may hold and still be replayed: `today` less the replay retention.
///
/// A day the calendar cannot read is a fault in the deployment's own clock rather than a source
/// condition, so this fails closed: a boundary derived from a date nobody can read would either
/// remove receipts that are still live or never remove any at all, and both are silent.
fn retention_boundary(today: &str) -> Result<String, HandlerError> {
    let day = NaiveDate::parse_from_str(today, "%Y-%m-%d")
        .map_err(|_| TerminalError::new(format!("sweep day {today} is not a YYYY-MM-DD day")))?;
    let boundary = day
        .checked_sub_days(Days::new(REPLAY_RETENTION_DAYS))
        .ok_or_else(|| {
            TerminalError::new("retention boundary underflows the calendar".to_string())
        })?;
    Ok(boundary.to_string())
}

/// One durable window wait, behind a seam: a `WorkflowContext` cannot be built in a unit test, so
/// the loop takes anything that can wait out a window. `true` = the window elapsed; `false` = the
/// stop signal cut it short.
trait WindowWaits {
    async fn window(&self, seconds: u64) -> bool;
}

impl WindowWaits for WorkflowContext<'_> {
    async fn window(&self, seconds: u64) -> bool {
        tokio::select! {
            _ = self.sleep(Duration::from_secs(seconds)) => true,
            _ = self.signal::<String>(STOP_SIGNAL) => false,
        }
    }
}

/// `Sweep`: observe the ingest objects across windows, durably sleeping between them.
#[derive(Clone)]
pub struct Sweep {
    store: Arc<Store>,
    clock: Arc<dyn Clock>,
    /// The shell's region: the report write runs through it, so a cancelled workflow leaves the
    /// region, not the runtime, owning the job.
    region: Arc<Spawner>,
}

impl Sweep {
    pub fn new(store: Arc<Store>, clock: Arc<dyn Clock>, region: Arc<Spawner>) -> Self {
        Self {
            store,
            clock,
            region,
        }
    }
}

#[workflow(
    journal_retention = "90 days",
    workflow_completion_retention = "180 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl Sweep {
    #[handler]
    #[tracing::instrument(
        skip_all,
        fields(
            windows = request.windows,
            endpoints = request.endpoints.len()
        )
    )]
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
        let today = super::journaled_today_workflow(&ctx, &self.clock).await?;
        let (windows_observed, interrupted) =
            Self::wait_windows(&ctx, request.windows, request.window_seconds).await?;
        let (endpoints, stale) = Self::observe_endpoints(&ctx, &request.endpoints).await?;
        let boundary = retention_boundary(&today)?;
        // Journaled under a single-attempt run policy (ADR-002): a restart replays the prune
        // count instead of deleting receipts a second time, and the invocation retry owns every
        // attempt after the first. `Json` is the bridge to the SDK's own serialization traits,
        // which is what `run` journals with.
        let Json(pruned) = ctx
            .run(|| async move {
                blocking_prune_receipts::blocking_prune_receipts(
                    Arc::clone(&self.region),
                    Arc::clone(&self.store),
                    boundary,
                )
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        let report = SweepReport {
            windows_observed,
            interrupted,
            endpoints,
            stale,
            today: today.clone(),
            report_path: None,
            pruned_receipts: pruned.removed,
            undated_receipts: pruned.undated,
        };
        let written = ctx
            .run(|| async move {
                blocking_write_report::blocking_write_report(
                    Arc::clone(&self.region),
                    Arc::clone(&self.store),
                    report,
                    today.clone(),
                )
                .await
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(written)
    }

    /// Sleep out `windows` durable windows, returning how many elapsed and whether the stop signal
    /// cut the wait short. Each window races `ctx.sleep(window_seconds)` against the stop signal, so
    /// a signal arriving mid-window ends the sweep there. `window_seconds == 0` means each wait is
    /// `Duration::ZERO`, which the runtime resolves immediately: the loop still waits, it just waits
    /// no time. A sweep that returns without its waits observes nothing.
    async fn wait_windows(
        ctx: &WorkflowContext<'_>,
        windows: u32,
        window_seconds: u64,
    ) -> Result<(u32, bool), HandlerError> {
        Ok(Self::wait_windows_with(ctx, windows, window_seconds).await)
    }

    /// The counting half of [`Self::wait_windows`]: how many windows elapsed, and whether a signal
    /// cut the sweep short. Split out so a test can drive it with a scripted window wait.
    async fn wait_windows_with<W: WindowWaits>(
        waits: &W,
        windows: u32,
        window_seconds: u64,
    ) -> (u32, bool) {
        let mut observed = 0_u32;
        for _ in 0..windows {
            if !waits.window(window_seconds).await {
                return (observed, true);
            }
            observed = observed.saturating_add(1);
        }
        (observed, false)
    }
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

mod wait_windows_tests;
