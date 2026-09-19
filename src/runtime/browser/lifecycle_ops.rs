use super::{Actor, BrowserError, BrowserState, BrowserStatus, NavigationOutcome};
use crate::runtime::browser::navigation;
use std::time::Instant;

/// Check if cooldown is currently active (until > now; poison => closed).
fn has_active_cooldown(cooldown_until: &std::sync::Arc<std::sync::Mutex<Option<Instant>>>) -> bool {
    match cooldown_until.lock() {
        Ok(value) => value.is_some_and(|until| until > Instant::now()),
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
        if has_active_cooldown(&self.cooldown_until) {
            return Ok(self.status());
        }
        let page = self
            .pages
            .first()
            .ok_or(BrowserError::Unavailable)?
            .page
            .clone();
        let outcome =
            match navigation::inspect(&page, &self.settings.source_origin, self.gate.clone()).await
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
        let target = self.challenge_target.as_ref().map_or_else(
            || self.settings.source_origin.clone(),
            |value| {
                if value.post {
                    self.settings.source_origin.clone()
                } else {
                    value.url.clone()
                }
            },
        );
        // Capture generation before async navigation.
        let generation_snapshot = self.gate.snapshot();
        // Compute active cooldown BEFORE try_open.
        if has_active_cooldown(&self.cooldown_until) {
            return Ok(self.status());
        }
        let outcome = if self.recovery_used {
            match navigation::inspect(&page, &self.settings.source_origin, self.gate.clone()).await
            {
                Ok(value) => value,
                Err(error) => {
                    self.gate.revoke();
                    self.challenge_latched = true;
                    self.set_state(BrowserState::Restarting);
                    return Err(error);
                }
            }
        } else {
            let target_clone = target.clone();
            let result = navigation::bootstrap(
                &page,
                &target_clone,
                self.settings.request_timeout,
                self.gate.clone(),
            )
            .await;
            // Set recovery_used AFTER successful bootstrap navigation.
            // This ensures we don't clear a challenge by inspecting an
            // unchanged Ready homepage before recovery.
            self.recovery_used = true;
            match result {
                Ok(value) => value,
                Err(error) => {
                    self.gate.revoke();
                    self.challenge_latched = true;
                    self.set_state(BrowserState::Restarting);
                    return Err(error);
                }
            }
        };
        let is_ready = matches!(outcome, NavigationOutcome::Ready);
        self.apply_navigation(outcome);
        // Only open the gate when the outcome is Ready and no concurrent revocation occurred.
        if !is_ready || !self.gate.try_open(generation_snapshot.generation) {
            return Ok(self.status());
        }
        // CAS successful — gate is now open, reset challenge_latched for next cycle.
        self.challenge_latched = false;
        // Finish remaining tabs after first-tab challenge resolves.
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
}
