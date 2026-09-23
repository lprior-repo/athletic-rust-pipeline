//! The single-attempt source workflow.
//!
//! `execute` runs one journaled step and publishes the final feedback once that step's verdict is
//! known. There is no attempt loop here: retrying belongs to the invocation, whose declared retry
//! policy is the run's only retry owner (ADR-002), so a second attempt is a second invocation
//! rather than a sleep inside this one. `WorkflowStep`/`StepError` are the protocol it shares with
//! `dispatch`, which resolves admission before the step.

use super::dispatch::ReadinessPolicy;
use super::{
    dispatch, result, AdmissionFeedback, SourceGateway, SourceGatewayClient, SOURCE_CONTROL_SCOPE,
};
use crate::runtime::http_audit;
use crate::runtime::protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence};
use athleticnet_browser::request::{RequestAction, RequestSpec};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) enum WorkflowStep {
    Deferred,
    Blocked { failure: OperationFailure },
    Attempt { finalized: result::Finalized },
}

pub(super) enum StepError {
    Admission(HandlerError),
    Effect(TerminalError),
}

pub(super) async fn execute(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let is_rankings = matches!(request.action, RequestAction::Rankings(_));
    let policy = ReadinessPolicy::from_request(&request);
    let operation = http_audit::operation_key(ctx.invocation_id(), "source-http")?;
    let step = match dispatch::admitted_step(gateway, ctx, &request, &operation, policy).await {
        Ok(step) => step,
        Err(StepError::Admission(error)) => return Err(error),
        Err(StepError::Effect(error)) if error.code() == 409 => return Err(error.into()),
        Err(_) => return Ok(uncertain_effect("source execution error")),
    };
    match step {
        WorkflowStep::Attempt { finalized } => {
            publish_final_feedback(ctx, &finalized, is_rankings).await?;
            Ok(finalized.outcome)
        }
        WorkflowStep::Blocked { failure } => Ok(FetchOutcome::Failed { failure }),
        // Admission resolves a Deferred ranking step to Blocked, so one arriving here escaped the
        // bound it is admitted under.
        WorkflowStep::Deferred => {
            Err(TerminalError::new("deferred browser step escaped admission").into())
        }
    }
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
