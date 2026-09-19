mod admission;
mod dispatch;
pub(crate) mod http;
pub(crate) mod observation;
pub(crate) mod request;
mod result;
pub(crate) mod retry;
use crate::runtime::{
    http_audit,
    protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource},
    Runtime,
};
pub use dispatch::ReadinessPolicy;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
#[cfg(test)]
mod tests;

pub use admission::{AdmissionDecision, AdmissionFeedback};

pub const SOURCE_SCOPE: &str = "athletic-source";
pub const SOURCE_CONCURRENCY: u32 = 16;
const SOURCE_CONTROL_SCOPE: &str = "athletic-source-control";
const SOURCE_ADMISSION_SCOPE: &str = "athletic-source-admission";

pub struct SourceGateway {
    pub runtime: Arc<Runtime>,
}

#[derive(Serialize, Deserialize)]
enum WorkflowStep {
    Deferred,
    Blocked {
        failure: OperationFailure,
    },
    Attempt {
        finalized: result::Finalized,
        retryable: bool,
        delay_ms: u64,
    },
}

enum StepError {
    Admission(HandlerError),
    Effect(TerminalError),
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
        admission::admit(&ctx, self.runtime.config.source_interval()).await
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
        admission::observe(&ctx, feedback.0).await
    }
}

async fn execute(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: crate::runtime::source::request::RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let is_rankings = matches!(
        request.action,
        crate::runtime::source::request::RequestAction::Rankings(_)
    );
    let policy = ReadinessPolicy::from_request(&request);
    let interval = gateway.runtime.config.source_interval();
    let operation = http_audit::operation_key(ctx.invocation_id(), "source-http")?;
    let mut last_finalized: Option<result::Finalized> = None;

    for attempt_index in 0..retry::MAX_ATTEMPTS {
        let step = match dispatch::admitted_step(
            gateway,
            ctx,
            &request,
            &operation,
            interval,
            attempt_index,
            policy,
        )
        .await
        {
            Ok(step) => step,
            Err(StepError::Admission(error)) => return Err(error),
            Err(StepError::Effect(error)) if error.code() == 409 => return Err(error.into()),
            Err(_) => {
                // Execution error: publish final feedback if we have finalized data,
                // then return the last known outcome or a generic failure.
                if let Some(finalized) = &last_finalized {
                    publish_final_feedback(ctx, finalized, is_rankings).await?;
                }
                let outcome = match last_finalized.take() {
                    Some(f) => f.outcome,
                    None => FetchOutcome::Failed {
                        failure: OperationFailure {
                            code: FailureCode::UncertainEffect,
                            message: "source execution error".into(),
                            http_status: None,
                            retries: RetryEvidence::NotAttempted,
                            evidence: Vec::new(),
                        },
                    },
                };
                return Ok(outcome);
            }
        };
        let (finalized, retryable, delay_ms) = match step {
            WorkflowStep::Attempt {
                finalized,
                retryable,
                delay_ms,
            } => (finalized, retryable, delay_ms),
            WorkflowStep::Blocked { failure } => {
                return Ok(
                    last_finalized.map_or(FetchOutcome::Failed { failure }, |value| value.outcome)
                );
            }
            WorkflowStep::Deferred => {
                return Err(TerminalError::new("deferred browser step escaped admission").into())
            }
        };
        let should_retry = retryable && attempt_index + 1 < retry::MAX_ATTEMPTS;
        last_finalized = Some(finalized);
        if !should_retry {
            break;
        }
        publish_feedback(ctx, None, delay_ms).await?;
        ctx.sleep(Duration::from_millis(delay_ms)).await?;
    }

    let finalized = match last_finalized {
        Some(finalized) => finalized,
        None => {
            return Ok(FetchOutcome::Failed {
                failure: OperationFailure {
                    code: FailureCode::UncertainEffect,
                    message: "no attempts completed".into(),
                    http_status: None,
                    retries: RetryEvidence::NotAttempted,
                    evidence: Vec::new(),
                },
            })
        }
    };
    publish_final_feedback(ctx, &finalized, is_rankings).await?;
    Ok(finalized.outcome)
}
async fn run_step(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: crate::runtime::source::request::RequestSpec,
    operation: crate::domain::identity::EvidenceDigest,
    interval: Duration,
    attempt_index: usize,
) -> Result<WorkflowStep, TerminalError> {
    let runtime = gateway.runtime.clone();
    let record_operation = operation.clone();
    let load_operation = operation.clone();
    let last_attempt = attempt_index + 1 == retry::MAX_ATTEMPTS;
    let is_rankings = matches!(
        request.action,
        crate::runtime::source::request::RequestAction::Rankings(_)
    );
    ctx.run(move || async move {
        let attempt = http::perform(runtime.clone(), &request).await;
        if attempt.code == Some(FailureCode::BrowserUnavailable) && !is_rankings {
            return Ok(Json(WorkflowStep::Deferred));
        }
        // Rankings: never retry HTTP failures (403/429/challenge).
        // Retain the actual receipt/evidence and return failure immediately.
        // Non-rankings requests keep the existing retryable behavior.
        let retryable = if is_rankings {
            false
        } else {
            attempt.retryable
        };
        let delay = if retryable {
            retry::next_delay(attempt_index, &attempt, interval).map_err(TerminalError::new)?
        } else {
            Duration::ZERO
        };
        let delay_ms = u64::try_from(delay.as_millis())
            .map_err(|_| TerminalError::new("source retry delay exceeds millisecond range"))?;
        let captured = observation::CapturedAttempt {
            request,
            result: attempt,
        };
        let digest = http_audit::record(runtime.clone(), record_operation, captured).await?;
        let records = http_audit::load(runtime, load_operation).await?;
        let finalized =
            result::finish_workflow(operation, records, Ok(Json(digest)), last_attempt)?;
        Ok(Json(WorkflowStep::Attempt {
            finalized,
            retryable,
            delay_ms,
        }))
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|json| json.0)
}

async fn publish_final_feedback(
    ctx: &SharedObjectContext<'_>,
    finalized: &result::Finalized,
    is_rankings: bool,
) -> Result<(), HandlerError> {
    if finalized.blocked && matches!(&finalized.outcome, FetchOutcome::Retrieved { .. }) {
        return Err(TerminalError::new("successful source response cannot block admission").into());
    }
    if is_rankings {
        return publish_feedback(ctx, None, finalized.cooldown_ms).await;
    }
    let failure = match (&finalized.outcome, finalized.blocked) {
        (FetchOutcome::Failed { failure }, true) => Some(failure.clone()),
        (FetchOutcome::Retrieved { .. }, true)
        | (FetchOutcome::Failed { .. } | FetchOutcome::Retrieved { .. }, false) => None,
    };
    publish_feedback(ctx, failure, finalized.cooldown_ms).await
}

async fn publish_feedback(
    ctx: &SharedObjectContext<'_>,
    failure: Option<OperationFailure>,
    cooldown_ms: u64,
) -> Result<(), HandlerError> {
    if failure.is_none() && cooldown_ms == 0 {
        return Ok(());
    }
    let feedback = ctx
        .object_client::<SourceGatewayClient>("global")
        .observe(Json(AdmissionFeedback {
            failure,
            cooldown_ms,
        }))
        .scope(SOURCE_CONTROL_SCOPE)
        .send()
        .await?;
    feedback.attach::<()>().await.map_err(Into::into)
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
