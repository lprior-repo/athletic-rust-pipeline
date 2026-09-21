use super::*;

/// Classify the selected attempt of a returned effect into its outcome and
/// whether the source is blocked for this collection.
pub(super) fn attempt_outcome(
    attempt: AttemptResult,
    retries: crate::runtime::protocol::RetryEvidence,
    mut evidence: Vec<crate::runtime::protocol::DocumentReceipt>,
    exhausted: bool,
) -> Result<(FetchOutcome, bool), HandlerError> {
    let retry_exhausted = exhausted && attempt.retryable;
    let challenged = attempt.code == Some(FailureCode::BrowserChallenge);
    let blocked = (retry_exhausted && (attempt.status == Some(429) || challenged))
        || (challenged && !attempt.retryable)
        || attempt.code == Some(FailureCode::AccessDenied)
        || (!challenged && matches!(attempt.status, Some(401 | 403)))
        || (matches!(attempt.status, Some(429 | 503))
            && !attempt.retryable
            && attempt.retry_after_ms == 0)
        || attempt.code == Some(FailureCode::ArtifactFailure);
    let code = retry_exhausted
        .then_some(FailureCode::RetryExhausted)
        .or(attempt.code);
    match code {
        None => {
            let receipt = attempt.receipt.ok_or_else(|| {
                TerminalError::new_with_code(
                    http_audit::AUDIT_FAILURE,
                    "successful source attempt is missing its receipt",
                )
            })?;
            Ok((
                FetchOutcome::Retrieved {
                    receipt,
                    retries,
                    previous_responses: evidence,
                },
                false,
            ))
        }
        Some(code) => {
            evidence.extend(attempt.receipt);
            Ok((
                FetchOutcome::Failed {
                    failure: OperationFailure {
                        code,
                        message: attempt.message,
                        http_status: attempt.status,
                        retries,
                        evidence,
                    },
                },
                blocked,
            ))
        }
    }
}

/// Classify a terminated source effect into its failure outcome and whether the
/// source is blocked for this collection.
pub(super) fn terminated_outcome(
    error: TerminalError,
    retries: crate::runtime::protocol::RetryEvidence,
    evidence: Vec<crate::runtime::protocol::DocumentReceipt>,
) -> (FetchOutcome, bool) {
    let code = if error.code() == http_audit::AUDIT_FAILURE {
        FailureCode::ArtifactFailure
    } else if error.code() == 500 {
        FailureCode::RetryExhausted
    } else {
        FailureCode::UncertainEffect
    };
    let blocked = code == FailureCode::ArtifactFailure;
    (
        FetchOutcome::Failed {
            failure: OperationFailure {
                code,
                message: format!(
                    "Restate source effect terminated with code {}; inspect retained attempt evidence",
                    error.code()
                ),
                http_status: None,
                retries,
                evidence,
            },
        },
        blocked,
    )
}
