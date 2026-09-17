mod admission;
mod body;
mod http;
mod request;
mod result;
pub(crate) mod retry;

use crate::runtime::{
    http_audit,
    protocol::{FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource},
    Runtime,
};
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

pub use admission::{AdmissionDecision, AdmissionFeedback};

pub const SOURCE_SCOPE: &str = "athletic-source";
pub const SOURCE_CONCURRENCY: u32 = 16;
const SOURCE_CONTROL_SCOPE: &str = "athletic-source-control";

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
        ctx: SharedObjectContext<'_>,
        input: Json<SourceResource>,
    ) -> Result<Json<FetchOutcome>, HandlerError> {
        if ctx.key() != "global" || ctx.scope() != Some(SOURCE_SCOPE) {
            return Err(TerminalError::new("invalid bounded source scope or key").into());
        }
        let request = match request::build(self.runtime.config.source_origin(), &input.0) {
            Ok(request) => request,
            Err(error) => {
                return Ok(Json(FetchOutcome::Failed {
                    failure: invalid(error.to_string()),
                }))
            }
        };
        if let Some(failure) = admission::wait(&ctx).await? {
            return Ok(Json(FetchOutcome::Failed { failure }));
        }
        execute(self, &ctx, request).await.map(Json)
    }

    #[handler]
    pub async fn admit(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<AdmissionDecision>, HandlerError> {
        admission::admit(&ctx, self.runtime.config.source_interval()).await
    }

    #[handler]
    pub async fn observe(
        &self,
        ctx: ObjectContext<'_>,
        feedback: Json<AdmissionFeedback>,
    ) -> Result<(), HandlerError> {
        admission::observe(&ctx, feedback.0).await
    }
}

async fn execute(
    gateway: &SourceGateway,
    ctx: &SharedObjectContext<'_>,
    request: request::RequestSpec,
) -> Result<FetchOutcome, HandlerError> {
    let interval = gateway.runtime.config.source_interval();
    let minimum_retry_delay = interval.max(Duration::from_secs(1));
    let maximum_retry_delay = minimum_retry_delay
        .checked_mul(4)
        .ok_or_else(|| TerminalError::new("source retry delay exceeds duration range"))?;
    let operation = http_audit::operation_key(ctx.invocation_id(), "source-http")?;
    // Native scope limits own concurrency; the SDK alone owns HTTP retries.
    let runtime = gateway.runtime.clone();
    let audit_operation = operation.clone();
    let effect = match ctx
        .run(|| async move {
            let mut attempt = http::perform(runtime.clone(), request).await;
            limit_retry_policy(&mut attempt);
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
        .await
    {
        Err(error) if error.code() == 409 => return Err(error.into()),
        effect => effect,
    };
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
        Err(error) if error.code() == 409 => return Err(error.into()),
        Err(_) => {
            let failure = OperationFailure {
                code: FailureCode::ArtifactFailure,
                message: "source evidence finalization failed; admission stopped pending repair"
                    .to_owned(),
                http_status: None,
                retries: http_audit::unavailable_evidence(operation)?,
                evidence: Vec::new(),
            };
            result::Finalized {
                outcome: FetchOutcome::Failed { failure },
                cooldown_ms: 0,
                blocked: true,
            }
        }
    };
    publish_feedback(ctx, &finalized).await?;
    Ok(finalized.outcome)
}

async fn publish_feedback(
    ctx: &SharedObjectContext<'_>,
    finalized: &result::Finalized,
) -> Result<(), HandlerError> {
    let failure = match (&finalized.outcome, finalized.blocked) {
        (FetchOutcome::Failed { failure }, true) => Some(failure.clone()),
        (FetchOutcome::Retrieved { .. }, true) => {
            return Err(
                TerminalError::new("successful source response cannot block admission").into(),
            );
        }
        (FetchOutcome::Failed { .. } | FetchOutcome::Retrieved { .. }, false) => None,
    };
    if failure.is_none() && finalized.cooldown_ms == 0 {
        return Ok(());
    }
    // Detach policy publication from the source caller's cancellation tree,
    // then wait for the policy to commit before reporting this outcome.
    let feedback = ctx
        .object_client::<SourceGatewayClient>("global")
        .observe(Json(AdmissionFeedback {
            failure,
            cooldown_ms: finalized.cooldown_ms,
        }))
        .scope(SOURCE_CONTROL_SCOPE)
        .send()
        .await?;
    feedback.attach::<()>().await.map_err(Into::into)
}

fn limit_retry_policy(attempt: &mut http::AttemptResult) {
    if attempt.retryable && attempt.retry_after_ms != 0 {
        attempt.retryable = false;
        attempt.message.push_str(
            "; Retry-After requires global admission cooldown; automatic retry stopped for review",
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
    fn positive_server_delay_stops_retries_without_discarding_delay() {
        let mut attempt = http::AttemptResult {
            receipt: None,
            code: Some(FailureCode::RateLimited),
            status: Some(429),
            message: String::new(),
            retryable: true,
            retry_after_ms: 1_000,
        };
        limit_retry_policy(&mut attempt);
        assert!(!attempt.retryable);
        assert_eq!(attempt.retry_after_ms, 1_000);
    }
}
