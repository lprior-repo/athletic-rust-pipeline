use super::{
    step::{self, EffectFailure},
    Runtime,
};
use crate::{
    discovery::{AthleticNetClient, SearchRequest},
    model::SearchHit,
    restate_types::row_key,
};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct SearchJob {
    pub digest: String,
    pub namespace: String,
    pub request: SearchRequest,
}
#[derive(Serialize, Deserialize)]
struct SourceReport {
    outcome: Result<Vec<SearchHit>, EffectFailure>,
    denials: u32,
    retry_after_millis: u64,
}

pub(super) struct AthleticSource {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    ingress_private = true,
    lazy_state = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 10, on_max_attempts = "pause")
)]
impl AthleticSource {
    #[handler]
    async fn search(
        &self,
        ctx: ObjectContext<'_>,
        job: Json<SearchJob>,
    ) -> Result<Json<Result<Vec<SearchHit>, EffectFailure>>, HandlerError> {
        self.runtime.validate(&job.0.digest)?;
        if ctx.key() != "athletic-source" {
            return Err(TerminalError::new("invalid source admission key").into());
        }
        let key = row_key(&job.0.namespace, &job.0.request.cache_key());
        if let Some(hits) = step::cached::<Vec<SearchHit>>(&ctx, &key).await? {
            return Ok(Json(Ok(hits)));
        }
        if ctx
            .get::<bool>("circuit-open")
            .await?
            .is_some_and(|open| open)
        {
            return Ok(Json(Err(EffectFailure::new(
                "SOURCE_CIRCUIT_OPEN",
                "source admission paused after denial; operator intervention required",
                false,
            ))));
        }
        let denials = ctx.get::<u32>("denials").await?.map_or(0, |value| value);
        let report = step::journal(&ctx, "search-query", || {
            self.execute(&job.0.request, denials)
        })
        .await?;
        let report = match report {
            Ok(report) => report,
            Err(error) => return Ok(Json(Err(error))),
        };
        let total = denials.saturating_add(report.denials);
        ctx.set("denials", total);
        let denied = report.outcome.as_ref().err().is_some_and(|error| {
            error.status == Some(403) || (error.status == Some(429) && !error.retryable)
        });
        if denied || total >= self.runtime.config.discovery.circuit_breaker_threshold {
            ctx.set("circuit-open", true);
        }
        // A final Retry-After delays the entire source queue, not only the failing row.
        if report.retry_after_millis > 0 {
            ctx.sleep(Duration::from_millis(report.retry_after_millis))
                .await?;
        }
        if let Ok(hits) = &report.outcome {
            ctx.set(&key, Json(hits.clone()));
        }
        Ok(Json(report.outcome))
    }
}

impl AthleticSource {
    async fn execute(
        &self,
        request: &SearchRequest,
        previous_denials: u32,
    ) -> Result<SourceReport, EffectFailure> {
        let mut config = self.runtime.config.discovery.clone();
        config.circuit_breaker_threshold = config
            .circuit_breaker_threshold
            .saturating_sub(previous_denials)
            .max(1);
        let client = AthleticNetClient::new(&config)
            .map_err(|error| EffectFailure::new("SOURCE_CONFIG", error.to_string(), false))?;
        // This admission wait deliberately runs again after an uncertain/crashed HTTP action.
        // The exclusive Restate object serializes all workers; pagination has its own same-rate gate.
        tokio::time::sleep(Duration::from_millis(config.search_delay_ms)).await;
        let result = client.execute_exhaustive(request).await;
        let (outcome, retry_after_millis) = match result {
            Ok(result) => (Ok(result.hits), 0),
            Err(error) => {
                let delay =
                    u64::try_from(error.retry_after.as_millis()).map_or(60_000, |value| value);
                (
                    Err(EffectFailure {
                        code: "SEARCH_ERROR".to_owned(),
                        message: error.message,
                        retryable: error.retryable,
                        status: error.status,
                    }),
                    delay,
                )
            }
        };
        Ok(SourceReport {
            outcome,
            denials: client.denial_count(),
            retry_after_millis,
        })
    }
}
