mod input;
#[cfg(feature = "fuzzing")]
pub mod model;
#[cfg(not(feature = "fuzzing"))]
mod model;
mod transport;

use self::{input::PreparedReview, model::Attempt, transport::request_once};
use super::{
    http_audit,
    protocol::{FailureCode, OperationFailure, ReviewJob, ReviewOutcome},
    ModelLane, Runtime,
};
use anyhow::anyhow;
use restate_sdk::prelude::*;
use serde::Serialize;
use std::{sync::Arc, time::Duration};

/// The local model reviewer. The object key is the exclusive model lane key.
pub struct LocalReviewer {
    pub runtime: Arc<Runtime>,
}

#[derive(Serialize, serde::Deserialize)]
struct FinalizedReview {
    outcome: ReviewOutcome,
    cooldown_ms: u64,
    blocked: bool,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "10m",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl LocalReviewer {
    #[handler]
    pub async fn review(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<ReviewJob>,
    ) -> Result<Json<ReviewOutcome>, HandlerError> {
        let job = job.into_inner();
        validate_lane(&ctx, job.lane)?;
        if let Some(failure) = ctx.get::<Json<OperationFailure>>("blocked").await? {
            return Ok(Json(ReviewOutcome::Failed {
                lane: job.lane,
                request: None,
                failure: failure.0,
            }));
        }
        let prepared = input::prepare(&self.runtime, &job)
            .await
            .map_err(terminal)?;
        let request = publish_request(&ctx, self.runtime.clone(), prepared.request.clone()).await?;
        let finalized = self
            .execute(&ctx, &prepared, request.clone(), job.lane)
            .await?;
        if finalized.cooldown_ms != 0 {
            ctx.sleep(Duration::from_millis(finalized.cooldown_ms))
                .await?;
        }
        if finalized.blocked {
            if let ReviewOutcome::Failed { failure, .. } = &finalized.outcome {
                ctx.set("blocked", Json(failure.clone()));
            }
        }
        Ok(Json(finalized.outcome))
    }

    async fn execute(
        &self,
        ctx: &ObjectContext<'_>,
        prepared: &PreparedReview,
        request: crate::domain::identity::EvidenceDigest,
        lane: ModelLane,
    ) -> Result<FinalizedReview, HandlerError> {
        let operation = http_audit::operation_key(ctx.invocation_id(), "local-review-http")?;
        let effect = run_attempt(
            ctx,
            self.runtime.clone(),
            operation.clone(),
            prepared.endpoint.clone(),
            prepared.request.clone(),
            prepared.input.clone(),
        )
        .await;
        let runtime = self.runtime.clone();
        let finalization_operation = operation.clone();
        let finalization_request = request.clone();
        let finalized = ctx
            .run(move || async move {
                let records = http_audit::load(runtime, finalization_operation.clone()).await?;
                finalize(
                    finalization_operation,
                    records,
                    effect,
                    finalization_request,
                    lane,
                )
                .map(Json)
            })
            .name("local-review-http-evidence-finalization")
            .retry_policy(RunRetryPolicy::new().max_attempts(4))
            .await;
        match finalized {
            Ok(value) => Ok(value.0),
            Err(_) => {
                let failure = OperationFailure {
                    code: FailureCode::ArtifactFailure,
                    message: "local model evidence finalization failed; model lane stopped pending repair".to_owned(),
                    http_status: None,
                    retries: http_audit::unavailable_evidence(operation)?,
                    evidence: Vec::new(),
                };
                Ok(FinalizedReview {
                    outcome: ReviewOutcome::Failed {
                        lane,
                        request: Some(request),
                        failure,
                    },
                    cooldown_ms: 0,
                    blocked: true,
                })
            }
        }
    }
}

async fn publish_request(
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

async fn run_attempt(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    operation: crate::domain::identity::EvidenceDigest,
    endpoint: String,
    request: model::ChatRequest,
    input: crate::runtime::protocol::ReviewInput,
) -> Result<Json<crate::domain::identity::EvidenceDigest>, TerminalError> {
    ctx.run(|| async move {
        let attempt = request_once(&runtime, &endpoint, &request, &input).await?;
        let retryable = attempt.retryable();
        let digest = http_audit::record(runtime, operation, attempt).await?;
        if retryable {
            Err(anyhow!("retryable local model outcome; evidence retained").into())
        } else {
            Ok(Json(digest))
        }
    })
    .name("local-review-http")
    .retry_policy(
        RunRetryPolicy::new()
            .initial_delay(Duration::from_secs(1))
            .exponentiation_factor(2.0)
            .max_delay(Duration::from_secs(4))
            .max_attempts(4),
    )
    .await
}

fn finalize(
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
                response: receipt,
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

fn validate_lane(ctx: &ObjectContext<'_>, lane: ModelLane) -> Result<(), HandlerError> {
    if ctx.key() == lane.key() {
        Ok(())
    } else {
        Err(terminal(anyhow!(
            "review object key does not match assigned model lane"
        )))
    }
}

fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> crate::domain::identity::EvidenceDigest {
        crate::domain::identity::EvidenceDigest::parse(&character.to_string().repeat(64))
            .expect("synthetic digest")
    }

    fn failed_record(character: char, cooldown_ms: u64) -> crate::store::AttemptEvidence<Attempt> {
        let response = crate::runtime::protocol::DocumentReceipt {
            digest: digest(character),
            source_url: "http://127.0.0.1:9000/v1/chat/completions".to_owned(),
            http_status: 503,
            media_type: "application/json".to_owned(),
            bytes: 1,
            fetched_at_unix_ms: 1,
            elapsed_ms: 1,
        };
        crate::store::AttemptEvidence {
            digest: digest(character),
            value: Attempt::Failure {
                code: FailureCode::HttpFailure,
                message: "synthetic transient failure".to_owned(),
                status: Some(503),
                receipt: Some(response),
                retryable: true,
                retry_after_ms: cooldown_ms,
            },
        }
    }

    #[test]
    fn sdk_exhaustion_retains_every_receipt_and_final_cooldown() {
        let records = vec![
            failed_record('a', 0),
            failed_record('b', 1_000),
            failed_record('c', 2_000),
        ];
        let finalized = finalize(
            digest('d'),
            records,
            Err(TerminalError::new("SDK exhausted")),
            digest('e'),
            ModelLane::Q5_5090,
        )
        .expect("synthetic finalization");
        assert_eq!(finalized.cooldown_ms, 2_000);
        let ReviewOutcome::Failed { failure, .. } = finalized.outcome else {
            panic!("SDK exhaustion must fail");
        };
        assert_eq!(failure.evidence.len(), 3);
        assert!(matches!(
            failure.retries,
            crate::runtime::protocol::RetryEvidence::SdkControlled {
                observed_attempts: 3,
                ..
            }
        ));
    }
}
