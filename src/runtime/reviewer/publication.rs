use super::{
    input,
    model::{self, Attempt},
    FinalizedReview,
};
use crate::domain::identity::EvidenceDigest;
use crate::runtime::protocol::{
    DocumentReceipt, FailureCode, OperationFailure, RetryEvidence, ReviewOutcome,
};
use crate::runtime::{http_audit, ModelLane, Runtime};
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) async fn publish_request(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    request: model::ChatRequest,
) -> Result<crate::domain::identity::EvidenceDigest, HandlerError> {
    Ok(ctx
        .run(|| async move {
            input::store_request(&runtime, &request)
                .await
                .map(Json)
                .map_err(terminal)
        })
        .name("local-review-request-publication")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?
        .0)
}

pub(super) fn finalize(
    operation: crate::domain::identity::EvidenceDigest,
    records: Vec<crate::store::AttemptEvidence<Attempt>>,
    effect: Result<Json<crate::domain::identity::EvidenceDigest>, TerminalError>,
    request: crate::domain::identity::EvidenceDigest,
    lane: ModelLane,
) -> Result<FinalizedReview, HandlerError> {
    let audit_failed = effect
        .as_ref()
        .err()
        .is_some_and(|error| error.code() == http_audit::AUDIT_FAILURE);
    let retries = if audit_failed {
        http_audit::unavailable_evidence(operation.clone())?
    } else {
        http_audit::retry_evidence(operation, &records)?
    };
    let returned = effect.as_ref().ok().map(|value| value.0.clone());
    let (selected, evidence, cooldown_ms) = select_evidence(&records, returned);
    let (outcome, blocked) = finalize_outcome(effect, selected, lane, request, retries, evidence)?;
    Ok(FinalizedReview {
        outcome,
        cooldown_ms,
        blocked,
    })
}

/// Splits the retained attempts into the one that carries the outcome, the remaining receipts, and
/// the longest cooldown any attempt asked for.
fn select_evidence(
    records: &[crate::store::AttemptEvidence<Attempt>],
    returned: Option<EvidenceDigest>,
) -> (Option<Attempt>, Vec<DocumentReceipt>, u64) {
    let selected_digest = returned.or_else(|| records.last().map(|record| record.digest.clone()));
    let selected = selected_digest.as_ref().and_then(|digest| {
        records
            .iter()
            .find(|record| record.digest == *digest)
            .map(|record| record.value.clone())
    });
    let evidence = records
        .iter()
        .filter(|record| Some(&record.digest) != selected_digest.as_ref())
        .filter_map(|record| record.value.receipt().cloned())
        .collect::<Vec<_>>();
    let cooldown_ms = records
        .iter()
        .map(|record| record.value.retry_after_ms())
        .fold(0, u64::max);
    (selected, evidence, cooldown_ms)
}

/// Maps the finalization state onto the review outcome and whether the model lane is blocked.
fn finalize_outcome(
    effect: Result<Json<EvidenceDigest>, TerminalError>,
    selected: Option<Attempt>,
    lane: ModelLane,
    request: EvidenceDigest,
    retries: RetryEvidence,
    evidence: Vec<DocumentReceipt>,
) -> Result<(ReviewOutcome, bool), HandlerError> {
    let selected = match effect {
        Ok(_) => selected,
        Err(error) => {
            return Ok(failed_finalization(
                error, selected, lane, request, retries, evidence,
            ))
        }
    };
    match selected {
        Some(Attempt::Success { verdict, receipt }) => Ok((
            ReviewOutcome::Reviewed {
                lane,
                verdict: verdict.into_protocol().map_err(terminal)?,
                request,
                response: Box::new(receipt),
                previous_responses: evidence,
                retries,
            },
            false,
        )),
        Some(Attempt::Failure {
            code,
            message,
            status,
            receipt,
            ..
        }) => Ok(failed_outcome(
            lane,
            request,
            code,
            message,
            status,
            retries,
            evidence.into_iter().chain(receipt).collect(),
        )),
        None => Err(TerminalError::new_with_code(
            http_audit::AUDIT_FAILURE,
            "acknowledged local model effect is absent from retained evidence",
        )
        .into()),
    }
}

/// Classifies an effect that ended in `error` against the attempt evidence it retained.
fn failed_finalization(
    error: TerminalError,
    selected: Option<Attempt>,
    lane: ModelLane,
    request: EvidenceDigest,
    retries: RetryEvidence,
    evidence: Vec<DocumentReceipt>,
) -> (ReviewOutcome, bool) {
    match selected {
        Some(Attempt::Failure {
            message,
            status,
            receipt,
            ..
        }) => failed_outcome(
            lane,
            request,
            effect_failure_code(&error),
            message,
            status,
            retries,
            evidence.into_iter().chain(receipt).collect(),
        ),
        Some(Attempt::Success { receipt, .. }) => failed_outcome(
            lane,
            request,
            uncertain_failure_code(&error),
            "Restate local model effect ended without an acknowledged result".to_owned(),
            Some(receipt.http_status),
            retries,
            evidence.into_iter().chain(Some(receipt)).collect(),
        ),
        None => failed_outcome(
            lane,
            request,
            uncertain_failure_code(&error),
            "Restate local model effect ended without retained attempt evidence".to_owned(),
            None,
            retries,
            evidence,
        ),
    }
}

/// Builds the failed review outcome; an artifact failure blocks the lane pending repair.
fn failed_outcome(
    lane: ModelLane,
    request: EvidenceDigest,
    code: FailureCode,
    message: String,
    http_status: Option<u16>,
    retries: RetryEvidence,
    evidence: Vec<DocumentReceipt>,
) -> (ReviewOutcome, bool) {
    let blocked = code == FailureCode::ArtifactFailure;
    (
        ReviewOutcome::Failed {
            lane,
            request: Some(request),
            failure: OperationFailure {
                code,
                message,
                http_status,
                retries,
                evidence,
            },
        },
        blocked,
    )
}

/// Failure code for an effect whose retained attempt was itself a failure.
fn effect_failure_code(error: &TerminalError) -> FailureCode {
    if error.code() == http_audit::AUDIT_FAILURE {
        FailureCode::ArtifactFailure
    } else if error.code() == 500 {
        FailureCode::RetryExhausted
    } else {
        FailureCode::UncertainEffect
    }
}

/// Failure code for an effect whose outcome the retained evidence cannot confirm.
fn uncertain_failure_code(error: &TerminalError) -> FailureCode {
    if error.code() == http_audit::AUDIT_FAILURE {
        FailureCode::ArtifactFailure
    } else {
        FailureCode::UncertainEffect
    }
}

pub(super) fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
