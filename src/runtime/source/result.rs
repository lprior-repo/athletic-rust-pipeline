use super::http::AttemptResult;
use crate::{
    domain::identity::EvidenceDigest,
    runtime::{
        http_audit,
        protocol::{FailureCode, FetchOutcome, OperationFailure},
    },
    store::AttemptEvidence,
};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Finalized {
    pub(super) outcome: FetchOutcome,
    pub(super) cooldown_ms: u64,
    pub(super) blocked: bool,
}

pub(super) fn finish_workflow(
    operation: EvidenceDigest,
    records: Vec<AttemptEvidence<AttemptResult>>,
    effect: Result<Json<EvidenceDigest>, TerminalError>,
    exhausted: bool,
) -> Result<Finalized, HandlerError> {
    let retries = if effect
        .as_ref()
        .is_err_and(|error| error.code() == http_audit::AUDIT_FAILURE)
    {
        http_audit::workflow_unavailable_evidence(operation)?
    } else {
        http_audit::workflow_retry_evidence(operation, &records)?
    };
    finish_with_retries(records, effect, retries, exhausted)
}

fn finish_with_retries(
    records: Vec<AttemptEvidence<AttemptResult>>,
    effect: Result<Json<EvidenceDigest>, TerminalError>,
    retries: crate::runtime::protocol::RetryEvidence,
    exhausted: bool,
) -> Result<Finalized, HandlerError> {
    let returned = effect.as_ref().ok().map(|value| &value.0);
    let mut selected = None;
    let mut evidence = Vec::new();
    let mut cooldown_ms = 0;
    for record in records {
        cooldown_ms = cooldown_ms.max(record.value.retry_after_ms);
        if returned == Some(&record.digest) {
            if selected.is_none() {
                selected = Some(record.value);
            } else if let Some(receipt) = record.value.receipt {
                evidence.push(receipt);
            }
        } else if let Some(receipt) = record.value.receipt {
            evidence.push(receipt);
        }
    }
    let selected_cooldown_ms = selected.as_ref().map(|attempt| attempt.retry_after_ms);
    let (outcome, blocked) = match (effect, selected) {
        (Ok(_), Some(attempt)) => {
            let retry_exhausted = exhausted && attempt.retryable;
            let blocked = (retry_exhausted && attempt.status == Some(429))
                || attempt.code == Some(FailureCode::AccessDenied)
                || matches!(attempt.status, Some(401 | 403))
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
                    (
                        FetchOutcome::Retrieved {
                            receipt,
                            retries,
                            previous_responses: evidence,
                        },
                        false,
                    )
                }
                Some(code) => {
                    evidence.extend(attempt.receipt);
                    (
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
                    )
                }
            }
        }
        (Err(error), _) => {
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
        (Ok(_), None) => {
            return Err(TerminalError::new_with_code(
                http_audit::AUDIT_FAILURE,
                "journaled source attempt is absent from retained evidence",
            )
            .into())
        }
    };
    let cooldown_ms = match outcome {
        FetchOutcome::Retrieved { .. } => 0,
        FetchOutcome::Failed { .. } => selected_cooldown_ms.map_or(cooldown_ms, |value| value),
    };
    Ok(Finalized {
        outcome,
        cooldown_ms,
        blocked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::protocol::{DocumentReceipt, RetryEvidence};

    fn digest(character: char) -> EvidenceDigest {
        EvidenceDigest::parse(&character.to_string().repeat(64)).expect("synthetic digest")
    }

    fn failed_attempt(character: char, cooldown: u64) -> AttemptEvidence<AttemptResult> {
        AttemptEvidence {
            digest: digest(character),
            value: AttemptResult {
                receipt: Some(DocumentReceipt {
                    digest: digest(character),
                    source_url: "http://127.0.0.1/fixture".into(),
                    http_status: 503,
                    media_type: "application/json".into(),
                    bytes: 1,
                    fetched_at_unix_ms: 1,
                    elapsed_ms: 1,
                }),
                code: Some(FailureCode::HttpFailure),
                status: Some(503),
                message: "synthetic failure".into(),
                retryable: true,
                retry_after_ms: cooldown,
            },
        }
    }
    #[test]
    fn access_denied_code_blocks_even_when_http_status_is_success() {
        let mut attempt = failed_attempt('a', 0);
        attempt.value.code = Some(FailureCode::AccessDenied);
        attempt.value.status = Some(200);
        attempt.value.receipt = Some(DocumentReceipt {
            digest: digest('a'),
            source_url: "http://127.0.0.1/fixture".into(),
            http_status: 200,
            media_type: "text/html".into(),
            bytes: 1,
            fetched_at_unix_ms: 1,
            elapsed_ms: 1,
        });
        let selected = attempt.digest.clone();
        let result = finish_workflow(digest('e'), vec![attempt], Ok(Json(selected)), false)
            .expect("finalization");
        assert!(result.blocked);
        let FetchOutcome::Failed { failure } = result.outcome else {
            panic!("expected access denial")
        };
        assert_eq!(failure.code, FailureCode::AccessDenied);
        assert_eq!(failure.http_status, Some(200));
    }

    #[test]
    fn workflow_exhaustion_retains_every_observed_receipt_and_final_cooldown() {
        let records = vec![
            failed_attempt('a', 0),
            failed_attempt('b', 1_000),
            failed_attempt('c', 500),
            failed_attempt('d', 2_000),
        ];
        let result = finish_workflow(
            digest('e'),
            records,
            Err(TerminalError::new("workflow exhausted")),
            true,
        )
        .expect("finalization");
        assert_eq!(result.cooldown_ms, 2_000);
        assert!(!result.blocked);
        let FetchOutcome::Failed { failure } = result.outcome else {
            panic!("expected failure")
        };
        assert_eq!(failure.code, FailureCode::RetryExhausted);
        assert_eq!(
            failure
                .evidence
                .iter()
                .map(|receipt| receipt.digest.clone())
                .collect::<Vec<_>>(),
            vec![digest('a'), digest('b'), digest('c'), digest('d')]
        );
        assert!(matches!(
            failure.retries,
            RetryEvidence::WorkflowControlled {
                observed_attempts: 4,
                ..
            }
        ));
    }

    #[test]
    fn unacknowledged_audit_write_never_claims_zero_http_attempts() -> anyhow::Result<()> {
        let result = finish_workflow(
            digest('e'),
            Vec::new(),
            Err(TerminalError::new_with_code(
                http_audit::AUDIT_FAILURE,
                "audit write failed",
            )),
            false,
        )
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        assert!(result.blocked);
        assert!(matches!(
            result.outcome,
            FetchOutcome::Failed {
                failure: OperationFailure {
                    retries: RetryEvidence::WorkflowEvidenceUnavailable { .. },
                    ..
                }
            }
        ));
        Ok(())
    }
}
