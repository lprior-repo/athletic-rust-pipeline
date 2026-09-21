use super::{Actor, BrowserError, BrowserState, BrowserStatus, NavigationOutcome};
use crate::runtime::browser::navigation;
use crate::runtime::clock::Clock;
use chromiumoxide::Page;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;
use url::Url;

/// Check if cooldown is currently active (until > now; poison => closed).
fn has_active_cooldown(clock: &dyn Clock, cooldown_until: &Arc<Mutex<Option<Instant>>>) -> bool {
    match cooldown_until.lock() {
        Ok(value) => value.is_some_and(|until| until > clock.now_instant()),
        Err(error) => error.into_inner().is_some(),
    }
}

impl Actor {
    pub(in crate::runtime::browser) async fn inspect_page(
        &mut self,
    ) -> Result<BrowserStatus, BrowserError> {
        // Return physical status for all non-ready states including CoolingDown,
        // even if the cooldown has expired. Do not navigate while CoolingDown.
        let current_state = self.status.read().ok().map(|s| s.state);
        if current_state == Some(BrowserState::CoolingDown) {
            return Ok(self.status());
        }
        if self.draining {
            return Ok(self.status());
        }
        if !self.jobs.is_empty() || (!self.gate.is_ready() && !self.recovery_used) {
            return Ok(self.status());
        }
        // Gate is closed but recovery_used — snapshot generation BEFORE async
        let snap = self.gate.snapshot();
        // Compute active cooldown BEFORE try_open to avoid opening then closing.
        if has_active_cooldown(self.clock.as_ref(), &self.cooldown_until) {
            return Ok(self.status());
        }
        let page = self
            .pages
            .first()
            .ok_or(BrowserError::Unavailable)?
            .page
            .clone();
        let outcome = match navigation::inspect(
            &page,
            &self.settings.source_origin,
            self.gate.clone(),
            self.clock.as_ref(),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                // Failed inspection: revoke admission and mark handled
                // so generic gate.closed does not turn invalid metadata
                // into a fresh automatic challenge navigation.
                self.gate.revoke();
                self.challenge_latched = true;
                self.set_state(BrowserState::Restarting);
                return Err(error);
            }
        };
        let is_ready = matches!(outcome, NavigationOutcome::Ready);
        self.apply_navigation(outcome);
        if !is_ready || !self.gate.try_open(snap.generation) {
            return Ok(self.status());
        }
        self.challenge_latched = false;
        Ok(self.status())
    }

    pub(in crate::runtime::browser) async fn recover_page(
        &mut self,
    ) -> Result<BrowserStatus, BrowserError> {
        // Never navigate with active jobs — SDK must skip Recover while
        // active_requests > 0.
        if !self.jobs.is_empty() {
            return Ok(self.status());
        }
        if self.draining {
            return Ok(self.status());
        }
        let page = self
            .pages
            .first()
            .ok_or(BrowserError::Unavailable)?
            .page
            .clone();
        let target = self.recovery_target();
        // Capture generation before async navigation.
        let generation_snapshot = self.gate.snapshot();
        // Compute active cooldown BEFORE try_open.
        if has_active_cooldown(self.clock.as_ref(), &self.cooldown_until) {
            return Ok(self.status());
        }
        let outcome = self.navigate_for_recovery(&page, &target).await?;
        let is_ready = matches!(outcome, NavigationOutcome::Ready);
        self.apply_navigation(outcome);
        // Only open the gate when the outcome is Ready and no concurrent revocation occurred.
        if !is_ready || !self.gate.try_open(generation_snapshot.generation) {
            return Ok(self.status());
        }
        // CAS successful — gate is now open, reset challenge_latched for next cycle.
        self.challenge_latched = false;
        self.complete_recovery_tabs().await
    }

    /// Target for the recovery navigation: a GET challenge target keeps its
    /// own URL, a POST one downgrades to the source origin.
    fn recovery_target(&self) -> Url {
        self.challenge_target.as_ref().map_or_else(
            || self.settings.source_origin.clone(),
            |value| {
                if value.post {
                    self.settings.source_origin.clone()
                } else {
                    value.url.clone()
                }
            },
        )
    }

    /// Navigate for recovery: inspect after a consumed latch, else bootstrap.
    ///
    /// A failed navigation revokes admission and latches the challenge before
    /// the error is handed back to the caller.
    async fn navigate_for_recovery(
        &mut self,
        page: &Page,
        target: &Url,
    ) -> Result<NavigationOutcome, BrowserError> {
        let outcome = if self.recovery_used {
            navigation::inspect(
                page,
                &self.settings.source_origin,
                self.gate.clone(),
                self.clock.as_ref(),
            )
            .await
        } else {
            let result = navigation::bootstrap(
                page,
                target,
                self.settings.request_timeout,
                self.gate.clone(),
                self.clock.as_ref(),
            )
            .await;
            // Set recovery_used AFTER successful bootstrap navigation.
            // This ensures we don't clear a challenge by inspecting an
            // unchanged Ready homepage before recovery.
            self.recovery_used = true;
            result
        };
        match outcome {
            Ok(value) => Ok(value),
            Err(error) => {
                self.gate.revoke();
                self.challenge_latched = true;
                self.set_state(BrowserState::Restarting);
                Err(error)
            }
        }
    }

    /// Finish remaining tabs after first-tab challenge resolves.
    async fn complete_recovery_tabs(&mut self) -> Result<BrowserStatus, BrowserError> {
        if self.gate.is_ready() && self.pages.len() < self.settings.tabs {
            if let Err(_error) = self.create_pages().await {
                self.gate.revoke();
                self.challenge_latched = true;
                self.set_state(BrowserState::Restarting);
                return Err(BrowserError::Transport);
            }
        }
        Ok(self.status())
    }

    /// Operator-requested relaunch: re-arm the one-shot recovery latch and
    /// re-run the bootstrap navigation, so a session whose latch was consumed
    /// can make progress again instead of reporting the stalled state forever.
    pub(in crate::runtime::browser) async fn restart_page(
        &mut self,
    ) -> Result<BrowserStatus, BrowserError> {
        self.recovery_used = false;
        self.challenge_latched = false;
        self.gate.revoke();
        self.set_state(BrowserState::Restarting);
        self.recover_page().await
    }
}
