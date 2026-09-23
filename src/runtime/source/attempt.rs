//! One journaled source attempt.
//!
//! `run_step` performs a single browser fetch inside `ctx.run`, classifies it, records the
//! observation in the HTTP audit, and finishes the workflow step. `receiptless_transport` holds
//! the rankings-specific rule: only a fault that left no receipt is worth an attempt by a later
//! invocation - this module never decides to ask the source again itself.

use super::http;
use super::observation;
use super::result;
use super::workflow::WorkflowStep;
use super::SourceGateway;
use crate::runtime::http_audit;
use crate::runtime::protocol::FailureCode;
use athleticnet_browser::request::{RequestAction, RequestSpec};
use restate_sdk::prelude::*;
use std::sync::Arc;

/// Whether a failed ranking attempt may be retried, given the fault code and
/// whether the attempt produced a receipt.
///
/// Only a receipt-less transport fault qualifies: nothing was observed, so
/// there is no evidence to preserve and no reason the same page cannot be
/// fetched once a later invocation is admitted.
pub(super) fn receiptless_transport(code: Option<FailureCode>, has_receipt: bool) -> bool {
    code == Some(FailureCode::Transport) && !has_receipt
}

pub(super) async fn run_step(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: RequestSpec,
    operation: crate::domain::identity::EvidenceDigest,
) -> Result<WorkflowStep, TerminalError> {
    let runtime = gateway.runtime.clone();
    let is_rankings = matches!(request.action, RequestAction::Rankings(_));
    ctx.run(move || async move {
        let attempt = http::perform(runtime.clone(), &request).await;
        if attempt.code == Some(FailureCode::BrowserUnavailable) && !is_rankings {
            return Ok(Json(WorkflowStep::Deferred));
        }
        let retryable = classify_attempt(&attempt, is_rankings);
        let captured = observation::CapturedAttempt {
            request,
            result: attempt,
        };
        journal_attempt(runtime, operation, captured, retryable).await
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|json| json.0)
}

/// Journal one attempt's evidence and finalize its workflow step.
///
/// The record/load pair is one journaled unit: the digest returned is the one the finalized step
/// carries, so a replay that loads the audit records sees exactly the attempt that was recorded.
///
/// The transport makes one attempt, so a classification that finds the fault retryable is already
/// reporting the exhausted verdict: `exhausted` is what tells the invocation retry this run has no
/// attempt left, and the failure is published with its cooldown instead of being slept off here.
async fn journal_attempt(
    runtime: Arc<crate::runtime::Runtime>,
    operation: crate::domain::identity::EvidenceDigest,
    captured: observation::CapturedAttempt,
    retryable: bool,
) -> Result<Json<WorkflowStep>, HandlerError> {
    let record_operation = operation.clone();
    let load_operation = operation.clone();
    let digest = http_audit::record(runtime.clone(), record_operation, captured).await?;
    let records = http_audit::load(runtime, load_operation).await?;
    let finalized = result::finish_workflow(operation, records, Ok(Json(digest)), retryable)?;
    Ok(Json(WorkflowStep::Attempt { finalized }))
}

/// Whether one attempt's fault is worth another invocation's attempt.
///
/// Rankings: never retry an attempt that observed the source.  A receipt
/// (403/429/challenge/parse failure) carries the evidence and is published
/// immediately, so the source is never hammered and no observation is
/// discarded.  A transport fault is a client-side fault: the browser client
/// lost the command response, no receipt exists, and the session's own
/// recovery navigation is what lets a later invocation make the fetch.
/// Non-rankings requests keep the transport's own retryable classification.
fn classify_attempt(attempt: &http::AttemptResult, is_rankings: bool) -> bool {
    if is_rankings {
        receiptless_transport(attempt.code, attempt.receipt.is_some())
    } else {
        attempt.retryable
    }
}
