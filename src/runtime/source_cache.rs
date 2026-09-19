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
        identity::scoped_key(
            &self.snapshot,
            &(super::acquisition::ACQUISITION_REVISION, &self.resource),
        )
    }

    /// Whether this request is for rankings.
    pub fn is_rankings(&self) -> bool {
        matches!(self.resource, SourceResource::Rankings { .. })
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
        let is_rankings = input.0.is_rankings();
        let outcome = ctx
            .object_client::<SourceGatewayClient>("global")
            .fetch(Json(input.0.resource))
            .scope(SOURCE_SCOPE)
            .call()
            .await?;
        // Cache successful outcomes always.
        // Cache failures only for non-rankings; rankings failures are not
        // cached so a resumed gateway invocation gets a fresh audit
        // identity and re-attempts the source rather than replaying a
        // cached failure.
        let should_cache = match &outcome.0 {
            FetchOutcome::Retrieved { .. } => true,
            FetchOutcome::Failed { .. } => !is_rankings,
        };
        if should_cache {
            ctx.set(
                "result",
                restate_sdk::serde::Serialize::serialize(&outcome)
                    .map_err(|error| TerminalError::new(error.to_string()))?,
            );
        }
        Ok(outcome)
    }
}
