//! The readiness observation cycle.
//!
//! One pass of `observe_ready` inspects the physical browser, enforces the durable cooldown, and
//! hands a non-ready state to `handle_challenge`; returning `None` asks the caller for another
//! observation after `POLL_INTERVAL`. `can_escalate` names the states that only an operator can
//! clear, which is why the cycle escalates to `HumanRequired` instead of polling to the deadline.
//!
//! The wall-clock reading for a pass is journaled once, by [`browser_readiness::now_ms`], and then
//! threaded as `now` through every guard below: a second read would add a second journal entry and
//! could observe a different millisecond than the deadline check it belongs to.

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
    let now = browser_readiness::now_ms(ctx, &runtime.clock()).await?;
    check_deadline(started, now)?;
    let status = recover_if_cooldown_expired(ctx, runtime.clone(), observed, now).await?;
    if let Some(ready) = record_ready(ctx, status.clone()) {
        return Ok(Some(ready));
    }
    let status = handle_challenge(ctx, runtime, status, now).await?;
    escalate_or_poll(ctx, status).await
}

/// Refuse to observe past the readiness deadline.
fn check_deadline(started: u64, now: u64) -> Result<(), HandlerError> {
    if now.saturating_sub(started) >= READINESS_DEADLINE_MS {
        return Err(TerminalError::new_with_code(
            408,
            "browser readiness deadline exhausted; operator action required",
        )
        .into());
    }
    Ok(())
}

/// Enforce the durable cooldown, then verify an expired one with a real navigation.
///
/// The navigation outcome is retained regardless of state: the recovery attempt is what is being
/// verified, not whether it reached `Ready`.
async fn recover_if_cooldown_expired(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    status: BrowserStatus,
    now: u64,
) -> Result<BrowserStatus, HandlerError> {
    let status = respect_cooldown(ctx, status, now).await?;
    // Cooldown expired — issue a fresh navigation to verify recovery
    // instead of reusing potentially-stale cached observation.
    if ctx.get::<bool>("cooldown-expired").await? != Some(true) {
        return Ok(status);
    }
    // Wait for active jobs to drain before issuing cooldown-expired Recover.
    let physical = browser_readiness::physical_status(&runtime).await;
    if physical.active_requests != 0 {
        return Ok(status);
    }
    let navigation = browser_readiness::act(ctx, runtime, BrowserAction::Recover).await?;
    if navigation.state == BrowserState::Ready {
        ctx.clear("cooldown-expired");
    }
    Ok(navigation)
}

/// Publish a `Ready` verdict and end the wait; `None` hands the status on for challenge handling.
fn record_ready(ctx: &ObjectContext<'_>, status: BrowserStatus) -> Option<BrowserStatus> {
    if status.state != BrowserState::Ready {
        return None;
    }
    ctx.clear("challenge-started-ms");
    ctx.clear("recovery-issued");
    ctx.clear("cooldown-expired");
    ctx.set("status", Json(status.clone()));
    Some(status)
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

/// Nothing to escalate: publish and end the wait.
async fn escalate_or_poll(
    ctx: &ObjectContext<'_>,
    status: BrowserStatus,
) -> Result<Option<BrowserStatus>, HandlerError> {
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

async fn handle_challenge(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    status: BrowserStatus,
    now: u64,
) -> Result<BrowserStatus, HandlerError> {
    if !can_escalate(status.state) {
        return Ok(status);
    }
    let started = challenge_started(ctx, now).await?;
    let status = issue_recovery_once(ctx, runtime.clone(), status).await?;
    escalate_after_window(ctx, runtime, status, started, now).await
}

/// The first observation that saw a challenge owns the escalation window.
async fn challenge_started(ctx: &ObjectContext<'_>, now: u64) -> Result<u64, HandlerError> {
    match ctx.get::<u64>("challenge-started-ms").await? {
        Some(value) => Ok(value),
        None => {
            ctx.set("challenge-started-ms", now);
            Ok(now)
        }
    }
}

/// Issue the one recovery navigation this challenge cycle is allowed.
///
/// Only when no active jobs and not cooling down; `recovery-issued` is marked AFTER the navigation
/// completes, regardless of outcome, because the navigation itself is the recovery attempt.
async fn issue_recovery_once(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    status: BrowserStatus,
) -> Result<BrowserStatus, HandlerError> {
    if status.state != BrowserState::Challenged
        || ctx.get::<bool>("recovery-issued").await? == Some(true)
    {
        return Ok(status);
    }
    let physical = browser_readiness::physical_status(&runtime).await;
    if physical.active_requests != 0 || physical.cooldown_ms != 0 {
        return Ok(status);
    }
    // Do NOT clear `recovery-issued` on duplicate challenge observations.
    let recovered = browser_readiness::act(ctx, runtime, BrowserAction::Recover).await?;
    ctx.set("recovery-issued", true);
    Ok(recovered)
}

/// Hand a still-unresolved challenge to the operator once its window has elapsed.
async fn escalate_after_window(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    status: BrowserStatus,
    started: u64,
    now: u64,
) -> Result<BrowserStatus, HandlerError> {
    let window = u64::try_from(runtime.config.browser_settings().challenge_wait.as_millis())
        .map_err(|_| TerminalError::new("browser challenge window overflow"))?;
    if status.state == BrowserState::Ready || now.saturating_sub(started) < window {
        return Ok(status);
    }
    browser_readiness::act(ctx, runtime, BrowserAction::HumanRequired).await
}

/// States that cannot recover without operator action: a challenge the human
/// must clear, or a restart attempt that never reached a verdict.
///
/// `pub(super)` so the session's tests can pin the escalation set.
pub(super) fn can_escalate(state: BrowserState) -> bool {
    matches!(state, BrowserState::Challenged | BrowserState::Restarting)
}
