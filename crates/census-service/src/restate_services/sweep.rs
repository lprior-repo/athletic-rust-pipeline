use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::spawn::Spawner;
use census_report::report::ReportResult;
use census_store::clock::Clock;
use census_store::Store;

use super::ingest::IngestClient;
use super::jobs::write_sweep_report;
use super::jobs::run_once;
use super::wire::ingest::{EndpointObservation, SweepReport, SweepRequest};
use super::{blocking, job_error, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS, STOP_SIGNAL};

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

        let mut report = SweepReport {
            windows_observed,
            interrupted,
            endpoints,
            stale,
            today: today.clone(),
            report_path: None,
        };
        let store = Arc::clone(&self.store);
        let region = Arc::clone(&self.region);
        let written = run_once(move || async move {
            // The closure's error type is spelled out: the report's typed error converts into a
            // `JobError` through `From`, and inference alone cannot choose between the two.
            blocking(region, move || -> ReportResult<Json<SweepReport>> {
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

#[cfg(test)]
mod wait_windows_tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// A scripted stand-in for the durable wait. It records every window it was asked to wait — the
    /// assertion that matters, since a loop that stopped waiting cannot fail a test that only counts
    /// windows.
    struct ScriptedWindows {
        cut_short_at: Option<u32>,
        calls: Cell<u32>,
        waited: RefCell<Vec<u64>>,
    }

    impl ScriptedWindows {
        fn new(cut_short_at: Option<u32>) -> Self {
            Self {
                cut_short_at,
                calls: Cell::new(0),
                waited: RefCell::new(Vec::new()),
            }
        }

        fn windows_waited(&self) -> Vec<u64> {
            self.waited.borrow().clone()
        }
    }

    impl WindowWaits for ScriptedWindows {
        async fn window(&self, seconds: u64) -> bool {
            let index = self.calls.get();
            self.calls.set(index.saturating_add(1));
            self.waited.borrow_mut().push(seconds);
            self.cut_short_at != Some(index)
        }
    }

    /// A zero-second window is a wait of zero length, not a skipped wait. The pre-fix code awaited
    /// `ctx.signal` alone for `window_seconds == 0`, so with no signal the future never resolved and
    /// the call never returned; a later repair deleted the sleep entirely. That is why this asserts
    /// the waits themselves: three windows must mean three waits.
    #[tokio::test]
    async fn a_zero_second_window_waits_once_per_window() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 0).await;

        assert_eq!(observed, 3, "all three windows are observed");
        assert!(!interrupted, "no signal arrived, so nothing was interrupted");
        assert_eq!(
            waits.windows_waited(),
            vec![0, 0, 0],
            "every window is waited out, even at zero seconds"
        );
    }

    /// A stop signal that has already arrived ends the sweep inside its first window.
    #[tokio::test]
    async fn an_arrived_signal_cuts_the_first_window_short() {
        let waits = ScriptedWindows::new(Some(0));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 10).await;

        assert_eq!(observed, 0);
        assert!(interrupted);
        assert_eq!(
            waits.windows_waited().len(),
            1,
            "the sweep stops inside the first window"
        );
    }

    /// A signal arriving mid-sweep keeps the windows already observed.
    #[tokio::test]
    async fn a_signal_mid_sweep_keeps_the_windows_observed_so_far() {
        let waits = ScriptedWindows::new(Some(2));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 30).await;

        assert_eq!(observed, 2);
        assert!(interrupted);
        assert_eq!(waits.windows_waited(), vec![30, 30, 30]);
    }

    /// Every window carries the requested duration, and exhausting them is not an interruption.
    #[tokio::test]
    async fn all_windows_elapse_without_a_signal() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 42).await;

        assert_eq!(observed, 3);
        assert!(!interrupted);
        assert_eq!(waits.windows_waited(), vec![42, 42, 42]);
    }

    /// No windows means no waits at all.
    #[tokio::test]
    async fn zero_windows_waits_for_nothing() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 0, 5).await;

        assert_eq!(observed, 0);
        assert!(!interrupted);
        assert!(waits.windows_waited().is_empty());
    }
}