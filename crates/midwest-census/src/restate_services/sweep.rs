use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::*;

use crate::bootstrap::Clock;
use crate::store::Store;

use super::ingest::IngestClient;
use super::jobs::write_sweep_report;
use super::wire::{EndpointObservation, SweepReport, SweepRequest};
use super::{blocking, job_error, MAX_SWEEP_ENDPOINTS, MAX_SWEEP_WINDOWS, STOP_SIGNAL};

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
