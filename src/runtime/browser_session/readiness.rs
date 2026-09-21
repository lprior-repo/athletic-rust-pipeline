//! The readiness observation cycle.
//!
//! One pass of `observe_ready` inspects the physical browser, enforces the durable cooldown, and
//! hands a non-ready state to `handle_challenge`; returning `None` asks the caller for another
//! observation after `POLL_INTERVAL`. `can_escalate` names the states that only an operator can
//! clear, which is why the cycle escalates to `HumanRequired` instead of polling to the deadline.

use super::super::browser::{BrowserState, BrowserStatus};
use super::super::browser_readiness::{self, BrowserAction};
use super::super::Runtime;
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const READINESS_DEADLINE_MS: u64 = 24 * 60 * 60 * 1_000;

pub(super) async fn observe_ready(
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
        let physical = browser_readiness::physical_status(&runtime).await;
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
    if status.state == BrowserState::HumanRequired {
        // Escalation ends this wait with an explicit durable state instead of
        // polling to the readiness deadline; the operator re-runs readiness
        // after clearing the challenge or relaunching the browser.
        ctx.set("status", Json(status.clone()));
        ctx.clear("challenge-started-ms");
        ctx.clear("recovery-issued");
        return Ok(Some(status));
    }
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
    if !can_escalate(status.state) {
        return Ok(status);
    }
    let started = match ctx.get::<u64>("challenge-started-ms").await? {
        Some(value) => value,
        None => {
            ctx.set("challenge-started-ms", now);
            now
        }
    };
    if status.state == BrowserState::Challenged
        && ctx.get::<bool>("recovery-issued").await? != Some(true)
    {
        // Only issue recovery when no active jobs and not cooling down.
        let physical = browser_readiness::physical_status(&runtime).await;
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

/// States that cannot recover without operator action: a challenge the human
/// must clear, or a restart attempt that never reached a verdict.
///
/// `pub(super)` so the session's tests can pin the escalation set.
pub(super) fn can_escalate(state: BrowserState) -> bool {
    matches!(state, BrowserState::Challenged | BrowserState::Restarting)
}
