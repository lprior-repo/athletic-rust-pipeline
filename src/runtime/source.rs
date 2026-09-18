mod admission;
mod body;
mod http;
pub(crate) mod observation;
pub(crate) mod request;
mod result;
pub(crate) mod retry;

use crate::runtime::{
    http_audit,
    protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource},
    Runtime,
};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub use admission::{AdmissionDecision, AdmissionFeedback};

pub const SOURCE_SCOPE: &str = "athletic-source";
pub const SOURCE_CONCURRENCY: u32 = 16;
const SOURCE_CONTROL_SCOPE: &str = "athletic-source-control";
const SOURCE_ADMISSION_SCOPE: &str = "athletic-source-admission";

pub struct SourceGateway {
    pub runtime: Arc<Runtime>,
}

#[derive(Serialize, Deserialize)]
struct WorkflowStep {
    finalized: result::Finalized,
    retryable: bool,
    delay_ms: u64,
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
        let request = match request::build(self.runtime.config.source_origin(), &input.0) {
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
    ) -> Result<Json<Option<OperationFailure>>, HandlerError> {
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
    request: request::RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let interval = gateway.runtime.config.source_interval();
    let operation = http_audit::operation_key(ctx.invocation_id(), "source-http")?;
    let mut last_finalized: Option<result::Finalized> = None;

    for attempt_index in 0..retry::MAX_ATTEMPTS {
        if let Some(failure) = admission::acquire(ctx).await? {
            return Ok(
                last_finalized.map_or(FetchOutcome::Failed { failure }, |value| value.outcome)
            );
        }
        let step = match run_step(
            gateway,
            ctx,
            request.clone(),
            operation.clone(),
            interval,
            attempt_index,
        )
        .await
        {
            Ok(step) => step,
            Err(error) if error.code() == 409 => return Err(error.into()),
            Err(_) => {
                let finalized = artifact_finalized(operation.clone(), last_finalized.as_ref())?;
                publish_final_feedback(ctx, &finalized).await?;
                return Ok(finalized.outcome);
            }
        };
        let should_retry = step.retryable && attempt_index + 1 < retry::MAX_ATTEMPTS;
        let delay_ms = step.delay_ms;
        let finalized = step.finalized;
        last_finalized = Some(finalized);
        if !should_retry {
            break;
        }
        publish_feedback(ctx, None, delay_ms).await?;
        ctx.sleep(Duration::from_millis(delay_ms)).await?;
    }

    let finalized = match last_finalized {
        Some(finalized) => finalized,
        None => artifact_finalized(operation, None)?,
    };
    publish_final_feedback(ctx, &finalized).await?;
    Ok(finalized.outcome)
}

async fn run_step(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: request::RequestSpec,
    operation: crate::domain::identity::EvidenceDigest,
    interval: Duration,
    attempt_index: usize,
) -> Result<WorkflowStep, TerminalError> {
    let runtime = gateway.runtime.clone();
    let record_operation = operation.clone();
    let load_operation = operation.clone();
    let last_attempt = attempt_index + 1 == retry::MAX_ATTEMPTS;
    ctx.run(move || async move {
        let attempt = http::perform(runtime.clone(), &request).await;
        let retryable = attempt.retryable;
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
        Ok(Json(WorkflowStep {
            finalized,
            retryable,
            delay_ms,
        }))
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|value| value.0)
}

fn artifact_finalized(
    operation: crate::domain::identity::EvidenceDigest,
    previous: Option<&result::Finalized>,
) -> Result<result::Finalized, HandlerError> {
    let evidence = match previous.map(|value| &value.outcome) {
        Some(FetchOutcome::Failed { failure }) => failure.evidence.clone(),
        Some(FetchOutcome::Retrieved {
            receipt,
            previous_responses,
            ..
        }) => {
            let mut evidence = previous_responses.clone();
            evidence.push(receipt.clone());
            evidence
        }
        None => Vec::new(),
    };
    Ok(result::Finalized {
        outcome: FetchOutcome::Failed {
            failure: OperationFailure {
                code: FailureCode::ArtifactFailure,
                message: "source evidence finalization failed; admission stopped pending repair"
                    .to_owned(),
                retries: http_audit::workflow_unavailable_evidence(operation)?,
                http_status: None,
                evidence,
            },
        },
        cooldown_ms: 0,
        blocked: true,
    })
}
async fn publish_final_feedback(
    ctx: &SharedObjectContext<'_>,
    finalized: &result::Finalized,
) -> Result<(), HandlerError> {
    let failure = match (&finalized.outcome, finalized.blocked) {
        (FetchOutcome::Failed { failure }, true) => Some(failure.clone()),
        (FetchOutcome::Retrieved { .. }, true) => {
            return Err(
                TerminalError::new("successful source response cannot block admission").into(),
            );
        }
        (FetchOutcome::Failed { .. } | FetchOutcome::Retrieved { .. }, false) => None,
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

#[cfg(test)]
mod tests;
