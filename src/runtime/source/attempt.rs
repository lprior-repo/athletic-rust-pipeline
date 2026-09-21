//! One journaled source attempt.
//!
//! `run_step` performs a single browser fetch inside `ctx.run`, classifies it for retry, records
//! the observation in the HTTP audit, and finishes the workflow step. `receiptless_transport` holds
//! the rankings-specific retry rule: only a fault that left no receipt is worth another attempt.

use super::http;
use super::observation;
use super::result;
use super::retry;
use super::workflow::WorkflowStep;
use super::SourceGateway;
use crate::runtime::http_audit;
use crate::runtime::protocol::FailureCode;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Whether a failed ranking attempt may be retried, given the fault code and
/// whether the attempt produced a receipt.
///
/// Only a receipt-less transport fault qualifies: nothing was observed, so
/// there is no evidence to preserve and no reason the same page cannot be
/// fetched again once the browser session is re-armed.
pub(super) fn receiptless_transport(code: Option<FailureCode>, has_receipt: bool) -> bool {
    code == Some(FailureCode::Transport) && !has_receipt
}

pub(super) async fn run_step(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: crate::runtime::source::request::RequestSpec,
    operation: crate::domain::identity::EvidenceDigest,
    interval: Duration,
    attempt_index: usize,
) -> Result<WorkflowStep, TerminalError> {
    let runtime = gateway.runtime.clone();
    // `attempt_index` is a retry ordinal bounded by `retry::MAX_ATTEMPTS`, so a
    // saturating increment is the exact attempt count and cannot overflow.
    let last_attempt = attempt_index.saturating_add(1) == retry::MAX_ATTEMPTS;
    let is_rankings = matches!(
        request.action,
        crate::runtime::source::request::RequestAction::Rankings(_)
    );
    ctx.run(move || async move {
        let attempt = http::perform(runtime.clone(), &request).await;
        if attempt.code == Some(FailureCode::BrowserUnavailable) && !is_rankings {
            return Ok(Json(WorkflowStep::Deferred));
        }
        let (retryable, delay_ms) =
            classify_attempt(&attempt, attempt_index, interval, is_rankings)?;
        let captured = observation::CapturedAttempt {
            request,
            result: attempt,
        };
        journal_attempt(
            runtime,
            operation,
            captured,
            retryable,
            delay_ms,
            last_attempt,
        )
        .await
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map(|json| json.0)
}

/// Journal one attempt's evidence and finalize its workflow step.
///
/// The record/load pair is one journaled unit: the digest returned is the one the finalized step
/// carries, so a replay that loads the audit records sees exactly the attempt that was recorded.
async fn journal_attempt(
    runtime: Arc<crate::runtime::Runtime>,
    operation: crate::domain::identity::EvidenceDigest,
    captured: observation::CapturedAttempt,
    retryable: bool,
    delay_ms: u64,
    last_attempt: bool,
) -> Result<Json<WorkflowStep>, HandlerError> {
    let record_operation = operation.clone();
    let load_operation = operation.clone();
    let digest = http_audit::record(runtime.clone(), record_operation, captured).await?;
    let records = http_audit::load(runtime, load_operation).await?;
    let finalized = result::finish_workflow(operation, records, Ok(Json(digest)), last_attempt)?;
    Ok(Json(WorkflowStep::Attempt {
        finalized,
        retryable,
        rearm: retryable,
        delay_ms,
    }))
}

/// Classify one attempt for retry and compute its pacing delay.
///
/// Rankings: never retry an attempt that observed the source.  A receipt
/// (403/429/challenge/parse failure) carries the evidence and is returned
/// immediately, so the source is never hammered and no observation is
/// discarded.  A transport fault is a client-side fault: the browser
/// client lost the command response, no receipt exists, and retrying it
/// after re-arming the session is what lets a desynced lane continue.
/// Non-rankings requests keep the existing retryable behavior.
///
/// The re-arm is not rankings-only: the same desynced client loses profile
/// fetches, and a non-rankings retry that reuses the session meets the same
/// fault until the row spends its attempt budget and the run parks.
fn classify_attempt(
    attempt: &http::AttemptResult,
    attempt_index: usize,
    interval: Duration,
    is_rankings: bool,
) -> Result<(bool, u64), TerminalError> {
    let retryable = if is_rankings {
        receiptless_transport(attempt.code, attempt.receipt.is_some())
    } else {
        attempt.retryable
    };
    let delay = if retryable {
        retry::next_delay(attempt_index, attempt, interval).map_err(TerminalError::new)?
    } else {
        Duration::ZERO
    };
    let delay_ms = u64::try_from(delay.as_millis())
        .map_err(|_| TerminalError::new("source retry delay exceeds millisecond range"))?;
    Ok((retryable, delay_ms))
}
