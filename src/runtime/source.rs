//! Bounded source gateway: the handlers that turn one `SourceResource` into one `FetchOutcome`.
//!
//! The object impl here owns the ingress surface only — scope check, request build, and the
//! admission/observation delegations. The retry workflow lives in [`workflow`], one journaled
//! attempt in `attempt`, and pacing in `admission`/`dispatch`.

mod admission;
mod attempt;
mod dispatch;
pub(crate) mod http;
pub(crate) mod observation;
pub(crate) mod request;
mod result;
pub(crate) mod retry;
mod workflow;
use crate::runtime::{
    protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource},
    Runtime,
};
pub use dispatch::ReadinessPolicy;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub use admission::{AdmissionDecision, AdmissionFeedback};

use attempt::run_step;
use workflow::{execute, StepError, WorkflowStep};

// `source/tests.rs` reaches both through `use super::*`: nothing in the production region reads
// `receiptless_transport` directly, and `Duration` is only needed to build retry-delay fixtures.
#[cfg(test)]
use attempt::receiptless_transport;
#[cfg(test)]
use std::time::Duration;

pub const SOURCE_SCOPE: &str = "athletic-source";
pub const SOURCE_CONCURRENCY: u32 = 16;
const SOURCE_CONTROL_SCOPE: &str = "athletic-source-control";
const SOURCE_ADMISSION_SCOPE: &str = "athletic-source-admission";

pub struct SourceGateway {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl SourceGateway {
    #[handler]
    pub async fn fetch(
        &self,
        ctx: SharedObjectContext<'_>,
        input: Json<SourceResource>,
    ) -> Result<Json<FetchOutcome>, HandlerError> {
        if ctx.key() != "global" || ctx.scope() != Some(SOURCE_SCOPE) {
            return Err(TerminalError::new("invalid bounded source scope or key").into());
        }
        let request = match crate::runtime::source::request::build(
            self.runtime.config.source_origin(),
            &input.0,
        ) {
            Ok(request) => request,
            Err(error) => {
                return Ok(Json(FetchOutcome::Failed {
                    failure: invalid(error.to_string()),
                }))
            }
        };
        execute(self, &ctx, request).await.map(Json)
    }

    #[handler]
    pub async fn admit(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<AdmissionDecision>, HandlerError> {
        admission::admit(
            &ctx,
            self.runtime.config.source_interval(),
            &self.runtime.clock(),
        )
        .await
    }

    #[handler]
    pub async fn await_admission(
        &self,
        ctx: ObjectContext<'_>,
        policy: Json<ReadinessPolicy>,
    ) -> Result<Json<Option<OperationFailure>>, HandlerError> {
        match policy.0 {
            ReadinessPolicy::Legacy => {
                // Non-rankings: wait for browser availability via the legacy
                // auto-recovery path, then wait for admission pacing.
                dispatch::await_browser(&ctx).await?;
            }
            ReadinessPolicy::Rankings => {
                // Rankings: one-shot readiness check via capture_ready.
                // Only BrowserState::Ready permits proceeding.
                // If not Ready, return blocked to collection immediately.
                use crate::runtime::browser::BrowserState;
                use crate::runtime::browser_session::{BrowserSessionClient, BROWSER_SESSION_KEY};
                let status = ctx
                    .object_client::<BrowserSessionClient>(BROWSER_SESSION_KEY)
                    .capture_ready()
                    .call()
                    .await?
                    .0;
                if status.state != BrowserState::Ready {
                    return Ok(Json(Some(OperationFailure {
                        code: FailureCode::BrowserUnavailable,
                        message: format!(
                            "browser not ready for rankings: state={:?}",
                            status.state
                        ),
                        http_status: None,
                        retries: RetryEvidence::NotAttempted,
                        evidence: Vec::new(),
                    })));
                }
            }
        }
        admission::wait(&ctx).await
    }

    #[handler]
    pub async fn observe(
        &self,
        ctx: ObjectContext<'_>,
        feedback: Json<AdmissionFeedback>,
    ) -> Result<(), HandlerError> {
        admission::observe(&ctx, feedback.0, &self.runtime.clock()).await
    }
}

fn invalid(message: String) -> OperationFailure {
    OperationFailure {
        code: FailureCode::InvalidInput,
        message,
        http_status: None,
        retries: RetryEvidence::NotAttempted,
        evidence: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
