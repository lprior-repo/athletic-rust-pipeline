use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_report::report::ReportResult;
use census_store::clock::Clock;
use census_store::Store;

use super::ingest::IngestClient;
use super::jobs::run_once;
use super::jobs::write_sweep_report;
use super::wire::ingest::{EndpointObservation, SweepReport, SweepRequest};
use super::{blocking, job_error, JobError, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS, STOP_SIGNAL};

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
        // Journaled: the date names the report file and travels inside the report the handler returns,
        // so a replay must reproduce it rather than re-read the clock.
        let today = super::journaled_today_workflow(&ctx, &self.clock).await?;
        let (windows_observed, interrupted) =
            Self::wait_windows(&ctx, request.windows, request.window_seconds).await?;
        let (endpoints, stale) = Self::observe_endpoints(&ctx, &request.endpoints).await?;

        let report = SweepReport {
            windows_observed,
            interrupted,
            endpoints,
            stale,
            today: today.clone(),
            report_path: None,
        };
        let written = run_once(|| async move {
            Self::blocking_write_report(
                Arc::clone(&self.region),
                Arc::clone(&self.store),
                report,
                today.clone(),
            )
            .await.map_err(job_error)
        })
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

    /// Persist the sweep report through the blocking pool, updating `report_path` on success.
    async fn blocking_write_report(
        spawner: Arc<Spawner>,
        store: Arc<Store>,
        report: SweepReport,
        today: String,
    ) -> Result<Json<SweepReport>, JobError> {
        blocking(spawner, move || -> ReportResult<Json<SweepReport>> {
            let path = write_sweep_report(&store, &report, &today)?;
            let mut report = report;
            report.report_path = Some(path.display().to_string());
            Ok(Json(report))
        })
        .await
    }
}

mod wait_windows_tests;
