mod input;
#[cfg(feature = "fuzzing")]
pub mod model;
#[cfg(not(feature = "fuzzing"))]
mod model;
mod transport;

mod dispatch;
mod publication;

use self::dispatch::{run_attempt, validate_lane};
use self::input::PreparedReview;
use self::publication::{finalize, publish_request, terminal};
use super::{
    http_audit,
    protocol::{FailureCode, OperationFailure, ReviewJob, ReviewOutcome},
    ModelLane, Runtime,
};
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
        let effect = match run_attempt(
            ctx,
            self.runtime.clone(),
            operation.clone(),
            prepared.endpoint.clone(),
            prepared.request.clone(),
            prepared.input.clone(),
        )
        .await
        {
            Err(error) if error.code() == 409 => return Err(error.into()),
            effect => effect,
        };
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
            Err(error) if error.code() == 409 => Err(error.into()),
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

#[cfg(test)]
mod tests;
