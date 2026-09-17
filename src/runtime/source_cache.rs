use super::{
    identity,
    protocol::{FetchOutcome, SourceResource},
    source::{SourceGatewayClient, SOURCE_SCOPE},
};
use crate::domain::identity::EvidenceDigest;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRequest {
    pub snapshot: EvidenceDigest,
    pub resource: SourceResource,
}

impl SourceRequest {
    pub fn key(&self) -> anyhow::Result<String> {
        identity::scoped_key(&self.snapshot, &self.resource)
    }
}

pub struct SourceCache;

#[restate_sdk::object(
    ingress_private = true,
    lazy_state = true,
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(max_attempts = 4, on_max_attempts = "pause")
)]
impl SourceCache {
    #[handler]
    async fn fetch(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<SourceRequest>,
    ) -> Result<Json<FetchOutcome>, HandlerError> {
        if input.0.key().terminal()? != ctx.key() {
            return Err(
                TerminalError::new("source cache key does not bind snapshot and resource").into(),
            );
        }
        if let Some(value) = ctx.get::<Json<FetchOutcome>>("result").await? {
            return Ok(value);
        }
        let outcome = ctx
            .object_client::<SourceGatewayClient>("global")
            .fetch(Json(input.0.resource))
            .scope(SOURCE_SCOPE)
            .call()
            .await?;
        ctx.set("result", Json(outcome.0.clone()));
        Ok(outcome)
    }
}
