use super::{
    input,
    model::{self, Attempt},
    FinalizedReview,
};
use crate::runtime::protocol::{FailureCode, OperationFailure, ReviewOutcome};
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
    let (outcome, blocked) = match (effect, selected) {
        (Ok(_), Some(Attempt::Success { verdict, receipt })) => (
            ReviewOutcome::Reviewed {
                lane,
                verdict: verdict.into_protocol().map_err(terminal)?,
                request,
                response: Box::new(receipt),
                previous_responses: evidence,
                retries,
            },
            false,
        ),
        (
            Ok(_),
            Some(Attempt::Failure {
                code,
                message,
                status,
                receipt,
                ..
            }),
        ) => (
            ReviewOutcome::Failed {
                lane,
                request: Some(request),
                failure: OperationFailure {
                    code,
                    message,
                    http_status: status,
                    retries,
                    evidence: evidence.into_iter().chain(receipt).collect(),
                },
            },
            code == FailureCode::ArtifactFailure,
        ),
        (
            Err(error),
            Some(Attempt::Failure {
                message,
                status,
                receipt,
                ..
            }),
        ) => {
            let code = if error.code() == http_audit::AUDIT_FAILURE {
                FailureCode::ArtifactFailure
            } else if error.code() == 500 {
                FailureCode::RetryExhausted
            } else {
                FailureCode::UncertainEffect
            };
            (
                ReviewOutcome::Failed {
                    lane,
                    request: Some(request),
                    failure: OperationFailure {
                        code,
                        message,
                        http_status: status,
                        retries,
                        evidence: evidence.into_iter().chain(receipt).collect(),
                    },
                },
                code == FailureCode::ArtifactFailure,
            )
        }
        (Err(error), Some(Attempt::Success { receipt, .. })) => {
            let code = if error.code() == http_audit::AUDIT_FAILURE {
                FailureCode::ArtifactFailure
            } else {
                FailureCode::UncertainEffect
            };
            (
                ReviewOutcome::Failed {
                    lane,
                    request: Some(request),
                    failure: OperationFailure {
                        code,
                        message: "Restate local model effect ended without an acknowledged result"
                            .to_owned(),
                        http_status: Some(receipt.http_status),
                        retries,
                        evidence: evidence.into_iter().chain(Some(receipt)).collect(),
                    },
                },
                code == FailureCode::ArtifactFailure,
            )
        }
        (Err(error), None) => {
            let code = if error.code() == http_audit::AUDIT_FAILURE {
                FailureCode::ArtifactFailure
            } else {
                FailureCode::UncertainEffect
            };
            (
                ReviewOutcome::Failed {
                    lane,
                    request: Some(request),
                    failure: OperationFailure {
                        code,
                        message:
                            "Restate local model effect ended without retained attempt evidence"
                                .to_owned(),
                        http_status: None,
                        retries,
                        evidence,
                    },
                },
                code == FailureCode::ArtifactFailure,
            )
        }
        (Ok(_), None) => {
            return Err(TerminalError::new_with_code(
                http_audit::AUDIT_FAILURE,
                "acknowledged local model effect is absent from retained evidence",
            )
            .into());
        }
    };
    Ok(FinalizedReview {
        outcome,
        cooldown_ms,
        blocked,
    })
}

pub(super) fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
