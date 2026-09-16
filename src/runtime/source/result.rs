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

pub(super) fn finish(
    operation: EvidenceDigest,
    records: Vec<AttemptEvidence<AttemptResult>>,
    effect: Result<Json<EvidenceDigest>, TerminalError>,
) -> Result<Finalized, HandlerError> {
    let retries = if effect
        .as_ref()
        .is_err_and(|error| error.code() == http_audit::AUDIT_FAILURE)
    {
        http_audit::unavailable_evidence(operation)?
    } else {
        http_audit::retry_evidence(operation, &records)?
    };
    let returned = effect.as_ref().ok().map(|value| &value.0);
    let mut selected = None;
    let mut evidence = Vec::new();
    let mut cooldown_ms = 0;
    for record in records {
        cooldown_ms = cooldown_ms.max(record.value.retry_after_ms);
        if returned == Some(&record.digest) {
            selected = Some(record.value);
        } else if let Some(receipt) = record.value.receipt {
            evidence.push(receipt);
        }
    }
    let (outcome, blocked) = match (effect, selected) {
        (Ok(_), Some(attempt)) => {
            let blocked = matches!(attempt.status, Some(401 | 403))
                || (matches!(attempt.status, Some(429 | 503))
                    && !attempt.retryable
                    && attempt.retry_after_ms == 0)
                || attempt.code == Some(FailureCode::ArtifactFailure);
            match attempt.code {
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
            (FetchOutcome::Failed { failure: OperationFailure { code,
                message: format!("Restate source effect terminated with code {}; inspect retained attempt evidence", error.code()),
                http_status: None, retries, evidence } }, blocked)
        }
        (Ok(_), None) => {
            return Err(TerminalError::new_with_code(
                http_audit::AUDIT_FAILURE,
                "journaled source attempt is absent from retained evidence",
            )
            .into())
        }
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
    fn sdk_exhaustion_retains_every_observed_receipt_and_final_cooldown() {
        let records = vec![
            failed_attempt('a', 0),
            failed_attempt('b', 1_000),
            failed_attempt('c', 500),
            failed_attempt('d', 2_000),
        ];
        let result = finish(
            digest('e'),
            records,
            Err(TerminalError::new("SDK exhausted")),
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
            RetryEvidence::SdkControlled {
                observed_attempts: 4,
                ..
            }
        ));
    }

    #[test]
    fn unacknowledged_audit_write_never_claims_zero_http_attempts() -> anyhow::Result<()> {
        let result = finish(
            digest('e'),
            Vec::new(),
            Err(TerminalError::new_with_code(
                http_audit::AUDIT_FAILURE,
                "audit write failed",
            )),
        )
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        assert!(result.blocked);
        assert!(matches!(
            result.outcome,
            FetchOutcome::Failed {
                failure: OperationFailure {
                    code: FailureCode::ArtifactFailure,
                    retries: RetryEvidence::SdkEvidenceUnavailable { .. },
                    ..
                }
            }
        ));
        Ok(())
    }
}
