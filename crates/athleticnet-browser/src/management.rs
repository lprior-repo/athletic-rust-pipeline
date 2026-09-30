use crate::navigation::NavigationOutcome;
use crate::retry::retry_after_now;
use crate::{
    actor::{Actor, ChallengeTarget},
    BrowserError, BrowserResponse, BrowserState,
};
use std::time::Duration;

#[path = "management/commands.rs"]
mod commands;
#[path = "management/pages.rs"]
mod pages;

impl Actor {
    pub(crate) fn complete_observer(
        &mut self,
        observer: Option<Result<Result<(), BrowserError>, tokio::task::JoinError>>,
    ) {
        match observer {
            Some(Ok(Ok(()))) if self.observer_stop.is_cancelled() => {}
            Some(Ok(Ok(()))) => {
                tracing::info!("browser observer completed unexpectedly (not cancelled)");
                self.set_state(BrowserState::Restarting);
                self.gate.revoke();
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.reject_pending(BrowserError::Unavailable);
            }
            Some(Ok(Err(e))) => {
                tracing::warn!(error = %e, "browser observer failed");
                self.set_state(BrowserState::Restarting);
                self.gate.revoke();
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.reject_pending(BrowserError::Unavailable);
            }
            Some(Err(join_err)) if join_err.is_panic() => {
                tracing::error!(
                    error = ?join_err,
                    "browser observer task panicked"
                );
                self.set_state(BrowserState::Restarting);
                self.gate.revoke();
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.reject_pending(BrowserError::Unavailable);
            }
            Some(Err(join_err)) if join_err.is_cancelled() => {
                tracing::debug!("browser observer task cancelled");
                self.set_state(BrowserState::Restarting);
                self.gate.revoke();
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.reject_pending(BrowserError::Unavailable);
            }
            Some(Err(join_err)) => {
                tracing::warn!(error = ?join_err, "browser observer terminated unexpectedly");
                self.set_state(BrowserState::Restarting);
                self.gate.revoke();
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.reject_pending(BrowserError::Unavailable);
            }
            None => {}
        }
    }

    pub(crate) fn latch_challenge(&mut self, request: Option<&crate::request::RequestSpec>) {
        if let Some(value) = request {
            self.challenge_target = Some(ChallengeTarget {
                url: value.url.clone(),
                post: value.body().is_some(),
            });
        }
        if !self.challenge_latched {
            self.recovery_used = false;
            self.challenge_latched = true;
        }
        self.gate.revoke();
        let is_cooling = self
            .status
            .read()
            .ok()
            .map(|s| s.state == BrowserState::CoolingDown)
            .unwrap_or(false);
        if !is_cooling {
            self.set_state(BrowserState::Challenged);
        }
        self.reject_pending(BrowserError::HumanRequired);
    }

    pub(crate) fn apply_navigation(&mut self, outcome: NavigationOutcome) {
        match outcome {
            NavigationOutcome::Ready => {
                let has_cooldown = match self.cooldown_until.lock() {
                    Ok(value) => value.is_some(),
                    Err(error) => error.into_inner().is_some(),
                };
                if !has_cooldown {
                    self.set_cooldown(Duration::ZERO);
                }
                self.set_state(BrowserState::Ready);
            }
            NavigationOutcome::Challenged => self.latch_challenge(None),
            NavigationOutcome::CoolingDown(delay) => self.apply_cooldown_duration(delay),
            NavigationOutcome::Pending => {
                self.gate.revoke();
                self.recovery_used = true;
                self.challenge_latched = true;
                if self
                    .status
                    .read()
                    .ok()
                    .map(|s| s.state == BrowserState::Ready)
                    .unwrap_or(false)
                {
                    self.set_state(BrowserState::Restarting);
                }
            }
            NavigationOutcome::Failed(_) => {
                self.gate.revoke();
                self.set_state(BrowserState::Restarting);
            }
        }
    }

    pub(crate) fn apply_cooldown(&mut self, response: &BrowserResponse) {
        match retry_after_now(self.clock.as_ref(), &response.headers) {
            Ok(delay) => self.apply_cooldown_duration(delay),
            Err(_) => {
                let delay = match response.headers.get("Retry-After") {
                    Some(value) => value
                        .to_str()
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(30),
                    None => 30,
                };
                self.apply_cooldown_duration(Duration::from_secs(delay));
            }
        }
    }

    pub(crate) fn apply_cooldown_duration(&mut self, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        let now = self.clock.now_instant();
        let until = match now.checked_add(delay) {
            Some(value) => value,
            None => now.checked_add(Duration::from_secs(86_400)).unwrap_or(now),
        };
        let current = match self.cooldown_until.lock() {
            Ok(value) => *value,
            Err(error) => *error.into_inner(),
        };
        let next = current.map_or(until, |value| value.max(until));
        match self.cooldown_until.lock() {
            Ok(mut value) => *value = Some(next),
            Err(error) => *error.into_inner() = Some(next),
        };
        self.recovery_used = false;
        self.gate.revoke();
        self.set_state(BrowserState::CoolingDown);
    }
}
