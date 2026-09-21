use super::{browser::BrowserStatus, browser_readiness, Runtime};
use futures::{StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod readiness;
mod recovery;
mod status;

use readiness::observe_ready;
use recovery::release_exhausted_recovery;
use status::{merge_status, validate_key};

pub const BROWSER_SESSION_KEY: &str = "profile-0";
const MAX_OBSERVATIONS: usize = 17_280;

/// Operator intent for the readiness workflow.
///
/// `operator` is true only when an operator explicitly re-ran readiness. That
/// call may re-arm an exhausted recovery latch exactly once; internal waiters
/// never clear a stall.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReadinessRequest {
    pub operator: bool,
}

pub struct BrowserSession {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    inactivity_timeout = "26h",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl BrowserSession {
    /// Restate, not the browser driver, owns challenge and human-wait policy.
    #[handler]
    pub async fn await_ready(
        &self,
        ctx: ObjectContext<'_>,
        request: Json<ReadinessRequest>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        if request.0.operator {
            release_exhausted_recovery(&ctx, self.runtime.clone()).await?;
        }
        let started = browser_readiness::now_ms(&ctx, &self.runtime.clock()).await?;
        let observations = futures::stream::iter(0..MAX_OBSERVATIONS)
            .then(|_| observe_ready(&ctx, self.runtime.clone(), started))
            .try_filter_map(|status| async { Ok(status) });
        futures::pin_mut!(observations);
        match observations.try_next().await? {
            Some(status) => Ok(Json(status)),
            None => Err(TerminalError::new_with_code(
                408,
                "browser readiness observation bound exhausted; operator action required",
            )
            .into()),
        }
    }

    #[handler]
    pub async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        let durable = ctx
            .get::<Json<BrowserStatus>>("status")
            .await?
            .map(|value| value.0);
        let runtime = self.runtime.clone();
        let physical = ctx
            .run(move || async move {
                Ok::<_, HandlerError>(Json(browser_readiness::physical_status(&runtime).await))
            })
            .name("observe physical browser status")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(merge_status(durable, physical)))
    }

    /// Capture the current browser readiness state under a shared context.
    ///
    /// Uses a bounded `ensure_browser` + one `Inspect` call.  Returns
    /// the observed status immediately — does NOT poll or wait for Ready.
    #[handler]
    pub async fn capture_ready(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        // Shared handler: must use ctx.run for physical acts.
        // ensure_browser + Inspect executed inside ctx.run boundary.
        let status = ctx
            .run(move || async move {
                let runtime = self.runtime.clone();
                let browser = runtime
                    .ensure_browser()
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok::<_, HandlerError>(Json(
                    browser
                        .inspect()
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?,
                ))
            })
            .name("capture_ready physical act")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(status))
    }

    /// Explicit shared-operator recovery with cooldown preservation.
    ///
    /// Issues a recovery navigation while keeping the current cooldown
    /// intact (the cooldown timer is NOT reset).  Returns the post-recovery
    /// status.
    #[handler]
    pub async fn recover(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        // Shared handler: must use ctx.run for physical acts.
        // ensure_browser + Recover executed inside ctx.run boundary.
        let status = ctx
            .run(move || async move {
                let runtime = self.runtime.clone();
                let browser = runtime
                    .ensure_browser()
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok::<_, HandlerError>(Json(
                    browser
                        .recover()
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?,
                ))
            })
            .name("recover physical act")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(status))
    }
}

#[cfg(test)]
mod tests;
