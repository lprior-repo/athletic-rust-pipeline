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
    #[tracing::instrument(skip_all, fields(key = %ctx.key(), lane = ?job.0.lane))]
    pub async fn review(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<ReviewJob>,
    ) -> Result<Json<ReviewOutcome>, HandlerError> {
        let job = job.into_inner();
        if let Some(outcome) = resolve_lane(&ctx, job.lane).await? {
            return Ok(outcome);
        }
        let prepared = input::prepare(&self.runtime, &job)
            .await
            .map_err(terminal)?;
        let request = publish_request(&ctx, self.runtime.clone(), prepared.request.clone()).await?;
        let finalized = self
            .execute(&ctx, &prepared, request.clone(), job.lane)
            .await?;
        apply_cooldown(&ctx, finalized.cooldown_ms).await?;
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
        let effect = self.run_effect(ctx, prepared, operation.clone()).await?;
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

    /// Runs one local model attempt. `Err` is the cancelled-lane signal; a model failure stays in
    /// the inner result, which `finalize` classifies from the retained attempt evidence.
    async fn run_effect(
        &self,
        ctx: &ObjectContext<'_>,
        prepared: &PreparedReview,
        operation: crate::domain::identity::EvidenceDigest,
    ) -> std::result::Result<
        std::result::Result<Json<crate::domain::identity::EvidenceDigest>, TerminalError>,
        TerminalError,
    > {
        match run_attempt(
            ctx,
            self.runtime.clone(),
            operation,
            prepared.endpoint.clone(),
            prepared.request.clone(),
            prepared.input.clone(),
        )
        .await
        {
            Err(error) if error.code() == 409 => Err(error),
            effect => Ok(effect),
        }
    }
}

#[cfg(test)]
mod tests;

/// Resolve the lane this reviewer serves, or the outcome that short-circuits a blocked lane.
///
/// A blocked lane answers with its durable failure instead of calling out again.
async fn resolve_lane(
    ctx: &ObjectContext<'_>,
    lane: ModelLane,
) -> Result<Option<Json<ReviewOutcome>>, HandlerError> {
    validate_lane(ctx, lane)?;
    let Some(failure) = ctx.get::<Json<OperationFailure>>("blocked").await? else {
        return Ok(None);
    };
    Ok(Some(Json(ReviewOutcome::Failed {
        lane,
        request: None,
        failure: failure.0,
    })))
}

/// Honour the lane's cadence before the next call.
async fn apply_cooldown(ctx: &ObjectContext<'_>, cooldown_ms: u64) -> Result<(), HandlerError> {
    if cooldown_ms == 0 {
        return Ok(());
    }
    ctx.sleep(Duration::from_millis(cooldown_ms)).await?;
    Ok(())
}
