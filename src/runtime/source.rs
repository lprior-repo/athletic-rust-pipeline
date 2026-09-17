mod http;
mod request;
mod result;
pub(crate) mod retry;
pub(super) mod spider_body;

use crate::runtime::{
    http_audit,
    protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource},
    Runtime,
};
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

pub struct SourceGateway {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl SourceGateway {
    #[handler]
    pub async fn fetch(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<SourceResource>,
    ) -> Result<Json<FetchOutcome>, HandlerError> {
        if ctx.key() != "global" {
            return Err(TerminalError::new("invalid source admission key").into());
        }
        if let Some(failure) = ctx.get::<Json<OperationFailure>>("blocked").await? {
            return Ok(Json(FetchOutcome::Failed { failure: failure.0 }));
        }
        let request = match request::build(self.runtime.config.source_origin(), &input.0) {
            Ok(request) => request,
            Err(error) => {
                return Ok(Json(FetchOutcome::Failed {
                    failure: invalid(error.to_string()),
                }))
            }
        };
        execute(self, &ctx, request).await.map(Json)
    }
}

async fn execute(
    gateway: &SourceGateway,
    ctx: &ObjectContext<'_>,
    request: request::RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let interval = gateway.runtime.config.source_interval();
    let minimum_retry_delay = interval.max(Duration::from_secs(1));
    let maximum_retry_delay = minimum_retry_delay
        .checked_mul(4)
        .ok_or_else(|| TerminalError::new("source retry delay exceeds duration range"))?;
    let operation = http_audit::operation_key(ctx.invocation_id(), "source-http")?;
    // Global object serialization and SDK timers own admission. The SDK alone owns retries.
    ctx.sleep(interval).await?;
    let runtime = gateway.runtime.clone();
    let audit_operation = operation.clone();
    let effect = ctx
        .run(|| async move {
            let mut attempt = http::perform(runtime.clone(), request).await;
            limit_retry_policy(&mut attempt, minimum_retry_delay);
            let retryable = attempt.retryable;
            let digest = http_audit::record(runtime, audit_operation, attempt).await?;
            if retryable {
                Err(anyhow::anyhow!("retryable source HTTP failure; evidence retained").into())
            } else {
                Ok(Json(digest))
            }
        })
        .name("source-http")
        .retry_policy(
            RunRetryPolicy::new()
                .initial_delay(minimum_retry_delay)
                .exponentiation_factor(2.0)
                .max_delay(maximum_retry_delay)
                .max_attempts(4),
        )
        .await;
    let runtime = gateway.runtime.clone();
    let finalization_operation = operation.clone();
    let finalized = ctx
        .run(|| async move {
            let records = http_audit::load(runtime, finalization_operation.clone()).await?;
            result::finish(finalization_operation, records, effect).map(Json)
        })
        .name("source-http-evidence-finalization")
        .retry_policy(RunRetryPolicy::new().max_attempts(4))
        .await;
    let finalized = match finalized {
        Ok(value) => value.0,
        Err(_) => {
            let failure = OperationFailure {
                code: FailureCode::ArtifactFailure,
                message: "source evidence finalization failed; admission stopped pending repair"
                    .to_owned(),
                http_status: None,
                retries: http_audit::unavailable_evidence(operation)?,
                evidence: Vec::new(),
            };
            ctx.set("blocked", Json(failure.clone()));
            return Ok(FetchOutcome::Failed { failure });
        }
    };
    // Honor the last failed response even after exhaustion, before the next caller enters.
    if finalized.cooldown_ms != 0 {
        ctx.sleep(Duration::from_millis(finalized.cooldown_ms))
            .await?;
    }
    if finalized.blocked {
        if let FetchOutcome::Failed { failure } = &finalized.outcome {
            ctx.set("blocked", Json(failure.clone()));
        }
    }
    Ok(finalized.outcome)
}

fn limit_retry_policy(attempt: &mut http::AttemptResult, minimum_delay: Duration) {
    if attempt.retryable && Duration::from_millis(attempt.retry_after_ms) > minimum_delay {
        attempt.retryable = false;
        attempt.message.push_str(
            "; Retry-After exceeds the SDK policy minimum; automatic retry stopped for review",
        );
    }
}

fn invalid(message: String) -> OperationFailure {
    OperationFailure {
        code: FailureCode::InvalidInput,
        message,
        http_status: None,
        retries: RetryEvidence::NotAttempted,
        evidence: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::evidence::Sport;
    use url::Url;

    #[test]
    fn team_route_retains_indoor_season_identifier() {
        let origin = Url::parse("http://127.0.0.1:9090/").expect("origin");
        let resource = SourceResource::Team {
            team_id: 7,
            sport: Sport::TrackField,
            season: 12025,
        };
        let request = request::build(&origin, &resource).expect("request");
        assert_eq!(request.url.path(), "/api/v1/TeamNav/Team");
        assert!(request
            .url
            .query_pairs()
            .any(|(key, value)| key == "season" && value == "12025"));
    }

    #[test]
    fn server_delay_beyond_sdk_policy_stops_retries_without_discarding_delay() {
        let mut attempt = http::AttemptResult {
            receipt: None,
            code: Some(FailureCode::RateLimited),
            status: Some(429),
            message: String::new(),
            retryable: true,
            retry_after_ms: 120_000,
        };
        limit_retry_policy(&mut attempt, Duration::from_secs(1));
        assert!(!attempt.retryable);
        assert_eq!(attempt.retry_after_ms, 120_000);
    }
}
