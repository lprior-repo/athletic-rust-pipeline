use super::publication::terminal;
use super::{model, transport::request_once};
use crate::runtime::{http_audit, ModelLane, Runtime};
use anyhow::anyhow;
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

pub(super) async fn run_attempt(
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
            .max_attempts(1),
    )
    .await
}

pub(super) fn validate_lane(ctx: &ObjectContext<'_>, lane: ModelLane) -> Result<(), HandlerError> {
    if ctx.key() == lane.key() {
        Ok(())
    } else {
        Err(terminal(anyhow!(
            "review object key does not match assigned model lane"
        )))
    }
}
