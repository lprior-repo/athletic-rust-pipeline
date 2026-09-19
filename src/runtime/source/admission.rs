use super::{dispatch::ReadinessPolicy, SourceGatewayClient, SOURCE_ADMISSION_SCOPE, SOURCE_CONTROL_SCOPE};
use crate::runtime::protocol::OperationFailure;
use futures::{StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionFeedback {
    pub failure: Option<OperationFailure>,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdmissionDecision {
    Granted,
    Deferred { wait_ms: u64 },
    Blocked { failure: OperationFailure },
}

pub(super) async fn admit(
    ctx: &ObjectContext<'_>,
    interval: Duration,
) -> Result<Json<AdmissionDecision>, HandlerError> {
    validate_control(ctx)?;
    if let Some(failure) = ctx.get::<Json<OperationFailure>>("blocked").await? {
        return Ok(Json(AdmissionDecision::Blocked { failure: failure.0 }));
    }
    if let Some(not_before) = ctx.get::<u64>("not-before-ms").await? {
        let remaining = not_before.saturating_sub(now_ms(ctx).await?);
        if remaining != 0 {
            return Ok(Json(AdmissionDecision::Deferred { wait_ms: remaining }));
        }
        ctx.clear("not-before-ms");
    }
    if !interval.is_zero() {
        let deadline = now_ms(ctx)
            .await?
            .checked_add(u64::try_from(interval.as_millis()).terminal()?)
            .ok_or_else(|| TerminalError::new("source admission deadline overflow"))?;
        ctx.set("not-before-ms", deadline);
    }
    Ok(Json(AdmissionDecision::Granted))
}

pub(super) async fn observe(
    ctx: &ObjectContext<'_>,
    feedback: AdmissionFeedback,
) -> Result<(), HandlerError> {
    validate_control(ctx)?;
    if let Some(failure) = feedback.failure {
        if ctx
            .get::<Json<OperationFailure>>("blocked")
            .await?
            .is_none()
        {
            ctx.set("blocked", Json(failure));
        }
    }
    // Persist the deadline before returning; cancelling this invocation cannot
    // discard an acknowledged cooldown. Source callers own durable waits.
    if feedback.cooldown_ms != 0 {
        let deadline = now_ms(ctx)
            .await?
            .checked_add(feedback.cooldown_ms)
            .ok_or_else(|| TerminalError::new("source cooldown deadline overflow"))?;
        let existing = ctx
            .get::<u64>("not-before-ms")
            .await?
            .map_or(0, |value| value);
        ctx.set("not-before-ms", deadline.max(existing));
    }
    Ok(())
}

fn validate_control(ctx: &ObjectContext<'_>) -> Result<(), HandlerError> {
    if ctx.key() != "global" || ctx.scope() != Some(SOURCE_CONTROL_SCOPE) {
        return Err(TerminalError::new("invalid source control scope or key").into());
    }
    Ok(())
}

async fn now_ms(ctx: &ObjectContext<'_>) -> Result<u64, HandlerError> {
    ctx.run(|| async {
        let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        Ok(u64::try_from(millis)?)
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(Into::into)
}

pub(super) async fn wait(
    ctx: &ObjectContext<'_>,
) -> Result<Json<Option<OperationFailure>>, HandlerError> {
    if ctx.key() != "global" || ctx.scope() != Some(SOURCE_ADMISSION_SCOPE) {
        return Err(TerminalError::new("invalid source admission scope or key").into());
    }
    // This exclusive queue owns the wait, not the control object. Feedback can
    // extend a cooldown while we sleep, but competing admissions cannot move
    // the pacing deadline. Only the at-most-15 already admitted peers can
    // extend it; the 64-observation guard detects a broken concurrency invariant.
    let decisions = futures::stream::iter(0..64)
        .then(|_| {
            ctx.object_client::<SourceGatewayClient>("global")
                .admit()
                .scope(SOURCE_CONTROL_SCOPE)
                .call()
        })
        .try_filter_map(|decision| async move {
            match decision.0 {
                AdmissionDecision::Granted => Ok(Some(None)),
                AdmissionDecision::Blocked { failure } => Ok(Some(Some(failure))),
                AdmissionDecision::Deferred { wait_ms } => {
                    ctx.sleep(Duration::from_millis(wait_ms)).await?;
                    Ok(None)
                }
            }
        });
    futures::pin_mut!(decisions);
    match decisions.try_next().await? {
        Some(outcome) => Ok(Json(outcome)),
        None => Err(anyhow::anyhow!(
            "serialized source admission exceeded its bounded in-flight feedback"
        )
        .into()),
    }
}

pub(super) async fn acquire(
    ctx: &SharedObjectContext<'_>,
    policy: ReadinessPolicy,
) -> Result<Option<OperationFailure>, HandlerError> {
    let call = ctx
        .object_client::<SourceGatewayClient>("global")
        .await_admission(restate_sdk::prelude::Json(policy))
        .scope(SOURCE_ADMISSION_SCOPE)
        .call();
    let handle = call.invocation_handle().await?;
    match call.await {
        Ok(value) => Ok(value.0),
        Err(error) if error.code() == 409 => {
            handle.cancel();
            Err(error.into())
        }
        Err(error) => Err(error.into()),
    }
}
