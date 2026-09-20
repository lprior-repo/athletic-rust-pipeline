use super::{admission, request, run_step, SourceGateway, StepError, WorkflowStep};
use futures::{StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Readiness policy: which browser readiness check to use.
/// Legacy: use the full auto-recovery path (await_ready).
/// Rankings: one-shot capture_ready, only Ready permits proceeding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessPolicy {
    Legacy,
    Rankings,
}

impl ReadinessPolicy {
    /// Derive the readiness policy from a request spec.
    pub(super) fn from_request(request: &request::RequestSpec) -> Self {
        match request.action {
            request::RequestAction::Rankings(_) => Self::Rankings,
            request::RequestAction::Fetch { .. } => Self::Legacy,
        }
    }
}

/// Wait for the browser to become available using the legacy auto-recovery
/// path.  Non-rankings requests use this because they may need the browser
/// to self-recover through challenge/human-wait cycles.
///
/// Measured 2026-09-20 (scale-v15): this exclusive single-key handler is the
/// per-lane throughput frame — it is entered exactly once per source operation,
/// its ≈0.34 s duration equals the observed operation period (2.98 ops/s), and
/// `tabs` (2→8) and `row_concurrency` (8→32) do not move it.  Probed a shared
/// `capture_ready` fast path first; it degrades to ≈1.55 ops/s whenever the
/// readiness bootstrap cannot answer (fixture lanes without a CDP endpoint), so
/// it is not shipped.  A replacement must answer without a physical act and
/// still leave this handler the only writer of readiness state.
pub(super) async fn await_browser(ctx: &ObjectContext<'_>) -> Result<(), HandlerError> {
    use crate::runtime::browser_session::{
        BrowserSessionClient, ReadinessRequest, BROWSER_SESSION_KEY,
    };
    let call = ctx
        .object_client::<BrowserSessionClient>(BROWSER_SESSION_KEY)
        .await_ready(Json(ReadinessRequest { operator: false }))
        .call();
    let handle = call.invocation_handle().await?;
    match call.await {
        Ok(_) => Ok(()),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error.into())
        }
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn admitted_step(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: &request::RequestSpec,
    operation: &crate::domain::identity::EvidenceDigest,
    interval: Duration,
    attempt_index: usize,
    policy: ReadinessPolicy,
) -> Result<WorkflowStep, StepError> {
    let steps = futures::stream::iter(0..64)
        .then(|_| async {
            if let Some(failure) = admission::acquire(ctx, policy)
                .await
                .map_err(StepError::Admission)?
            {
                return Ok(Some(WorkflowStep::Blocked { failure }));
            }
            let step = run_step(
                gateway,
                ctx,
                request.clone(),
                operation.clone(),
                interval,
                attempt_index,
            )
            .await
            .map_err(StepError::Effect)?;
            // Rankings: if the gate races closed after admission, Deferred
            // becomes an immediate Blocked — not a 64-iteration retry loop.
            // Legacy retains existing loops.
            match step {
                WorkflowStep::Deferred if policy == ReadinessPolicy::Rankings => {
                    Ok(Some(WorkflowStep::Blocked {
                        failure: crate::runtime::protocol::OperationFailure {
                            code: crate::runtime::protocol::FailureCode::BrowserUnavailable,
                            message: "browser admission closed after readiness check; rankings gate raced".into(),
                            http_status: None,
                            retries: crate::runtime::protocol::RetryEvidence::NotAttempted,
                            evidence: Vec::new(),
                        },
                    }))
                }
                WorkflowStep::Deferred => {
                    ctx.sleep(Duration::from_secs(1))
                        .await
                        .map_err(|error| StepError::Admission(error.into()))?;
                    Ok::<_, StepError>(None)
                }
                other => Ok(Some(other)),
            }
        })
        .try_filter_map(|step| async { Ok(step) });
    futures::pin_mut!(steps);
    steps.try_next().await?.ok_or_else(|| {
        StepError::Admission(TerminalError::new("browser admission race bound exhausted").into())
    })
}
