//! The bounded retry workflow.
//!
//! `execute` walks at most `retry::MAX_ATTEMPTS` journaled steps, re-arming the browser session and
//! pacing the next attempt through admission feedback, and publishes the final feedback once the
//! last finalized step is known. `WorkflowStep`/`StepError` are the protocol it shares with
//! `dispatch`, which resolves admission before each `run_step`.

use super::dispatch::ReadinessPolicy;
use super::{
    dispatch, result, retry, AdmissionFeedback, SourceGateway, SourceGatewayClient,
    SOURCE_CONTROL_SCOPE,
};
use crate::runtime::http_audit;
use crate::runtime::protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence};
use crate::runtime::source::request::RequestAction;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub(super) enum WorkflowStep {
    Deferred,
    Blocked {
        failure: OperationFailure,
    },
    Attempt {
        finalized: result::Finalized,
        retryable: bool,
        #[serde(default)]
        rearm: bool,
        delay_ms: u64,
    },
}

pub(super) enum StepError {
    Admission(HandlerError),
    Effect(TerminalError),
}

/// Re-arm the browser session before retrying a receipt-less transport fault.
///
/// A retry against the desynced client meets the same failure; the session's
/// recovery navigation is what restores the page and preserves the cooldown.
async fn rearm_browser_session(ctx: &SharedObjectContext<'_>) -> Result<(), HandlerError> {
    use crate::runtime::browser_session::{BrowserSessionClient, BROWSER_SESSION_KEY};
    ctx.object_client::<BrowserSessionClient>(BROWSER_SESSION_KEY)
        .recover()
        .call()
        .await?;
    Ok(())
}

/// Pace the next attempt: re-arm the session when the step asked for it, publish the delay as
/// admission feedback, then sleep it out.
async fn pace_attempt(
    ctx: &SharedObjectContext<'_>,
    rearm: bool,
    delay_ms: u64,
) -> Result<(), HandlerError> {
    if rearm {
        rearm_browser_session(ctx).await?;
    }
    publish_feedback(ctx, None, delay_ms).await?;
    ctx.sleep(Duration::from_millis(delay_ms)).await?;
    Ok(())
}

pub(super) async fn execute(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: crate::runtime::source::request::RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let is_rankings = matches!(request.action, RequestAction::Rankings(_));
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
                return consume_finalized(ctx, is_rankings, last_finalized.take()).await;
            }
        };
        match decide(step, attempt_index)? {
            StepDecision::Blocked(failure) => {
                return Ok(
                    last_finalized.map_or(FetchOutcome::Failed { failure }, |value| value.outcome)
                );
            }
            StepDecision::Finish(finalized) => {
                last_finalized = Some(finalized);
                break;
            }
            StepDecision::Pacing {
                finalized,
                rearm,
                delay_ms,
            } => {
                last_finalized = Some(finalized);
                pace_attempt(ctx, rearm, delay_ms).await?;
            }
        }
    }

    let finalized = match last_finalized {
        Some(finalized) => finalized,
        None => return Ok(uncertain_effect("no attempts completed")),
    };
    publish_final_feedback(ctx, &finalized, is_rankings).await?;
    Ok(finalized.outcome)
}

/// What the driver does with one resolved step.
enum StepDecision {
    /// Stop the workflow with this step's outcome.
    Finish(result::Finalized),
    /// Keep the step, re-arm the session when asked, and pace the next attempt.
    Pacing {
        finalized: result::Finalized,
        rearm: bool,
        delay_ms: u64,
    },
    /// Admission blocked the workflow; stop with the retained outcome.
    Blocked(OperationFailure),
}

/// Resolve one admitted step into a decision.
///
/// A blocked step ends the workflow, and an attempt past the retry budget is final even when it was
/// classified retryable. `attempt_index` stays below `MAX_ATTEMPTS` here, so saturation keeps the
/// increment checked without changing the comparison.
fn decide(step: WorkflowStep, attempt_index: usize) -> Result<StepDecision, HandlerError> {
    match step {
        WorkflowStep::Attempt {
            finalized,
            retryable,
            rearm,
            delay_ms,
        } => {
            if retryable && attempt_index.saturating_add(1) < retry::MAX_ATTEMPTS {
                Ok(StepDecision::Pacing {
                    finalized,
                    rearm,
                    delay_ms,
                })
            } else {
                Ok(StepDecision::Finish(finalized))
            }
        }
        WorkflowStep::Blocked { failure } => Ok(StepDecision::Blocked(failure)),
        WorkflowStep::Deferred => {
            Err(TerminalError::new("deferred browser step escaped admission").into())
        }
    }
}

/// Stop the workflow after a step resolved to an execution error.
///
/// Publishes the retained step's feedback and returns its outcome; with nothing retained the
/// failure is reported as an uncertain effect.
async fn consume_finalized(
    ctx: &SharedObjectContext<'_>,
    is_rankings: bool,
    last_finalized: Option<result::Finalized>,
) -> Result<FetchOutcome, HandlerError> {
    if let Some(finalized) = &last_finalized {
        publish_final_feedback(ctx, finalized, is_rankings).await?;
    }
    Ok(last_finalized.map_or_else(
        || uncertain_effect("source execution error"),
        |finalized| finalized.outcome,
    ))
}

/// The terminal failure for a workflow that ended without an observed verdict.
fn uncertain_effect(message: &str) -> FetchOutcome {
    FetchOutcome::Failed {
        failure: OperationFailure {
            code: FailureCode::UncertainEffect,
            message: message.to_string(),
            http_status: None,
            retries: RetryEvidence::NotAttempted,
            evidence: Vec::new(),
        },
    }
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
