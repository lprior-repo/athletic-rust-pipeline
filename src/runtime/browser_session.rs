use super::{
    browser::{BrowserState, BrowserStatus},
    browser_readiness::{self, BrowserAction},
    Runtime,
};
use futures::{StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

pub const BROWSER_SESSION_KEY: &str = "profile-0";
const MAX_OBSERVATIONS: usize = 17_280;
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const READINESS_DEADLINE_MS: u64 = 24 * 60 * 60 * 1_000;

pub struct BrowserSession {
    pub runtime: Arc<Runtime>,
}

#[restate_sdk::object(
    inactivity_timeout = "26h",
    journal_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl BrowserSession {
    /// Restate, not the browser driver, owns challenge and human-wait policy.
    #[handler]
    pub async fn await_ready(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        let started = browser_readiness::now_ms(&ctx).await?;
        let observations = futures::stream::iter(0..MAX_OBSERVATIONS)
            .then(|_| observe_ready(&ctx, self.runtime.clone(), started))
            .try_filter_map(|status| async { Ok(status) });
        futures::pin_mut!(observations);
        match observations.try_next().await? {
            Some(status) => Ok(Json(status)),
            None => Err(TerminalError::new_with_code(
                408,
                "browser readiness observation bound exhausted; operator action required",
            )
            .into()),
        }
    }

    #[handler]
    pub async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        let durable = ctx
            .get::<Json<BrowserStatus>>("status")
            .await?
            .map(|value| value.0);
        let runtime = self.runtime.clone();
        let physical = ctx
            .run(move || async move {
                Ok::<_, HandlerError>(Json(browser_readiness::physical_status(&runtime)))
            })
            .name("observe physical browser status")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(merge_status(durable, physical)))
    }

    /// Capture the current browser readiness state under a shared context.
    ///
    /// Uses a bounded `ensure_browser` + one `Inspect` call.  Returns
    /// the observed status immediately — does NOT poll or wait for Ready.
    #[handler]
    pub async fn capture_ready(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        // Shared handler: must use ctx.run for physical acts.
        // ensure_browser + Inspect executed inside ctx.run boundary.
        let status = ctx
            .run(move || async move {
                let runtime = self.runtime.clone();
                let browser = runtime
                    .ensure_browser()
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok::<_, HandlerError>(Json(
                    browser
                        .inspect()
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?,
                ))
            })
            .name("capture_ready physical act")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(status))
    }

    /// Explicit shared-operator recovery with cooldown preservation.
    ///
    /// Issues a recovery navigation while keeping the current cooldown
    /// intact (the cooldown timer is NOT reset).  Returns the post-recovery
    /// status.
    #[handler]
    pub async fn recover(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserStatus>, HandlerError> {
        validate_key(ctx.key())?;
        // Shared handler: must use ctx.run for physical acts.
        // ensure_browser + Recover executed inside ctx.run boundary.
        let status = ctx
            .run(move || async move {
                let runtime = self.runtime.clone();
                let browser = runtime
                    .ensure_browser()
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok::<_, HandlerError>(Json(
                    browser
                        .recover()
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?,
                ))
            })
            .name("recover physical act")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?
            .0;
        Ok(Json(status))
    }
}

async fn observe_ready(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    started: u64,
) -> Result<Option<BrowserStatus>, HandlerError> {
    let observed = browser_readiness::act(ctx, runtime.clone(), BrowserAction::Inspect).await?;
    let now = browser_readiness::now_ms(ctx).await?;
    if now.saturating_sub(started) >= READINESS_DEADLINE_MS {
        return Err(TerminalError::new_with_code(
            408,
            "browser readiness deadline exhausted; operator action required",
        )
        .into());
    }
    let status = respect_cooldown(ctx, observed, now).await?;
    // Cooldown expired — issue a fresh navigation to verify recovery
    // instead of reusing potentially-stale cached observation.
    let status = if ctx.get::<bool>("cooldown-expired").await? == Some(true) {
        // Wait for active jobs to drain before issuing cooldown-expired Recover.
        let physical = browser_readiness::physical_status(&runtime);
        if physical.active_requests == 0 {
            let navigation =
                browser_readiness::act(ctx, runtime.clone(), BrowserAction::Recover).await?;
            // Retain the navigation outcome regardless of state — verify
            // the navigation actually occurred, not just whether it reached Ready.
            if navigation.state == BrowserState::Ready {
                ctx.clear("cooldown-expired");
            }
            navigation
        } else {
            status
        }
    } else {
        status
    };
    if status.state == BrowserState::Ready {
        ctx.clear("challenge-started-ms");
        ctx.clear("recovery-issued");
        ctx.clear("cooldown-expired");
        ctx.set("status", Json(status.clone()));
        return Ok(Some(status));
    }
    let status = handle_challenge(ctx, runtime, status, now).await?;
    ctx.set("status", Json(status));
    ctx.sleep(POLL_INTERVAL).await?;
    Ok(None)
}

async fn respect_cooldown(
    ctx: &ObjectContext<'_>,
    mut status: BrowserStatus,
    now: u64,
) -> Result<BrowserStatus, HandlerError> {
    let previous = ctx
        .get::<u64>("cooldown-until-ms")
        .await?
        .map_or(0, |value| value);
    let observed = now
        .checked_add(status.cooldown_ms)
        .ok_or_else(|| TerminalError::new("browser cooldown deadline overflow"))?;
    let deadline = previous.max(observed);
    if deadline > now {
        ctx.set("cooldown-until-ms", deadline);
        ctx.clear("cooldown-expired");
        status.cooldown_ms = deadline.saturating_sub(now);
        if status.state == BrowserState::Ready {
            status.state = BrowserState::CoolingDown;
        }
    } else if previous > 0 {
        ctx.clear("cooldown-until-ms");
        ctx.set("cooldown-expired", true);
    }
    Ok(status)
}

async fn handle_challenge(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    mut status: BrowserStatus,
    now: u64,
) -> Result<BrowserStatus, HandlerError> {
    if !matches!(
        status.state,
        BrowserState::Challenged | BrowserState::HumanRequired
    ) {
        return Ok(status);
    }
    let started = match ctx.get::<u64>("challenge-started-ms").await? {
        Some(value) => value,
        None => {
            ctx.set("challenge-started-ms", now);
            now
        }
    };
    if ctx.get::<bool>("recovery-issued").await? != Some(true) {
        // Only issue recovery when no active jobs and not cooling down.
        let physical = browser_readiness::physical_status(&runtime);
        if physical.active_requests == 0 && physical.cooldown_ms == 0 {
            // Issue recovery navigation. Mark recovery-issued AFTER the
            // navigation completes regardless of outcome — the navigation
            // itself is the recovery attempt, not just a Ready result.
            status = browser_readiness::act(ctx, runtime.clone(), BrowserAction::Recover).await?;
            // Only set recovery-issued once per challenge cycle.
            // Do NOT clear it on duplicate challenge observations.
            ctx.set("recovery-issued", true);
        }
    }
    let window = u64::try_from(runtime.config.browser_settings().challenge_wait.as_millis())
        .map_err(|_| TerminalError::new("browser challenge window overflow"))?;
    if status.state != BrowserState::Ready && now.saturating_sub(started) >= window {
        status = browser_readiness::act(ctx, runtime, BrowserAction::HumanRequired).await?;
    }
    Ok(status)
}

fn merge_status(durable: Option<BrowserStatus>, mut physical: BrowserStatus) -> BrowserStatus {
    if let Some(durable) = durable {
        if physical.state == BrowserState::Ready && durable.state != BrowserState::Ready {
            physical.state = durable.state;
            physical.cooldown_ms = durable.cooldown_ms;
        }
    }
    physical
}

fn validate_key(key: &str) -> Result<(), HandlerError> {
    if key != BROWSER_SESSION_KEY {
        return Err(TerminalError::new("invalid browser session key").into());
    }
    Ok(())
}
