use crate::drain::{count, DrainReport};
use crate::navigation::{self, NavigationOutcome};
use crate::retry::retry_after_now;
use crate::{
    actor::{Actor, ChallengeTarget, Command},
    pool::{self, PageSlot},
    BrowserError, BrowserResponse, BrowserState,
};
use chromiumoxide::cdp::browser_protocol::target::{CreateTargetParams, TargetId};
use std::{future::Future, time::Duration};
use tokio::sync::oneshot;
use tracing::Instrument;

/// Send one command's result back, logging a receiver that went away.
///
/// A dropped reply is not an error: the caller was cancelled, and the work it asked for is done
/// either way.
async fn reply_with<T>(
    label: &'static str,
    reply: oneshot::Sender<T>,
    result: impl Future<Output = T>,
) {
    if reply.send(result.await).is_err() {
        tracing::debug!(reply = label, "browser command reply dropped");
    }
}

impl Actor {
    pub(crate) fn complete_observer(
        &mut self,
        observer: Option<Result<Result<(), BrowserError>, tokio::task::JoinError>>,
    ) {
        match observer {
            Some(Ok(Ok(()))) if self.observer_stop.is_cancelled() => {}
            Some(Ok(Ok(()))) | Some(Ok(Err(_))) | Some(Err(_)) => {
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
            // A deadline the platform clock cannot represent must not panic;
            // bound it to the longest representable fallback instead.
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

    /// Handle one command, or the closed command channel.
    ///
    /// Each arm is one ingress call: the four that answer a caller go through `reply_with`, and the
    /// two drain entries share `begin_drain`.
    pub(crate) async fn command(&mut self, command: Option<Command>) {
        match command {
            Some(Command::Bootstrap { reply }) => {
                reply_with("bootstrap", reply, self.bootstrap()).await;
            }
            Some(Command::Fetch { request, reply }) => self.accept_fetch(request, reply),
            Some(Command::Inspect { reply }) => {
                reply_with("inspect", reply, self.inspect_page()).await;
            }
            Some(Command::Recover { reply }) => {
                reply_with("recover", reply, self.recover_page()).await;
            }
            Some(Command::Restart { reply }) => {
                reply_with("restart", reply, self.restart_page()).await;
            }
            Some(Command::Shutdown { reply }) => self.begin_drain(Some(reply)),
            None => self.begin_drain(None),
        }
    }

    /// Start the drain: keep the reply that carries the region certificate, then revoke every
    /// admission path. The `Shutdown` command and a closed command channel are the same drain.
    fn begin_drain(&mut self, reply: Option<oneshot::Sender<DrainReport>>) {
        if let Some(reply) = reply {
            self.shutdown_reply = Some(reply);
        }
        self.draining = true;
        self.shutdown.cancel();
        self.observer_stop.cancel();
        self.gate.revoke();
        self.reject_pending(BrowserError::Shutdown);
    }

    /// Reject every queued request, counting each as accepted-then-cancelled.
    ///
    /// A queued request is a unit this actor took responsibility for, so whichever path rejects it
    /// the certificate says so; the caller learns the reason from its own reply channel.
    pub(crate) fn reject_pending(&mut self, cause: BrowserError) {
        let rejected = count(self.pending.len());
        self.region.accept(rejected);
        self.region.cancel(rejected);
        pool::reject_pending(&mut self.pending, cause);
    }

    pub(crate) async fn create_pages(&mut self) -> anyhow::Result<()> {
        while self.pages.len() < self.settings.tabs {
            let browser = self
                .browser
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("browser unavailable"))?;
            let params = CreateTargetParams::builder()
                .url("about:blank")
                .background(true)
                .build()
                .map_err(|_| anyhow::anyhow!("create target params failed"))?;
            let page = browser
                .new_page(params)
                .await
                .map_err(|_| anyhow::anyhow!("browser page creation failed"))?;
            self.pages.push(PageSlot::new(page));
            self.update_tab_count();
            let page = self
                .pages
                .last()
                .ok_or_else(|| anyhow::anyhow!("browser page missing"))?
                .page
                .clone();
            let generation_snapshot = self.gate.snapshot();
            let observer = navigation::start_observer(
                page.clone(),
                self.observer_stop.clone(),
                self.gate.clone(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("browser page observer failed to start"))?;
            self.observers
                .spawn(observer.run().instrument(tracing::info_span!(
                    "browser.observer",
                    target = %page.target_id().inner()
                )));
            let outcome = navigation::bootstrap(
                &page,
                &self.settings.source_origin,
                self.settings.request_timeout,
                self.gate.clone(),
                self.clock.as_ref(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("browser bootstrap failed"))?;
            self.apply_navigation(outcome);
            if self.gate.try_open(generation_snapshot.generation) {
                self.challenge_latched = false;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// A launched Chromium restores the profile's previous tabs, so a relaunch
    /// accumulates pages this pool never tracks. They keep loading and compete
    /// with the pool for renderer capacity, which raises the odds that the next
    /// acquisition fails and the following recovery adds another one. Attached
    /// (loopback) sessions share a browser this process did not start, so their
    /// pages are left alone.
    async fn close_restored_pages(&mut self) -> anyhow::Result<()> {
        let tracked: std::collections::HashSet<TargetId> = self
            .pages
            .iter()
            .map(|slot| slot.page.target_id().clone())
            .collect();
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("browser unavailable"))?;
        let pages = browser
            .pages()
            .await
            .map_err(|_| anyhow::anyhow!("browser page list failed"))?;
        for page in pages {
            if !tracked.contains(page.target_id()) && page.close().await.is_err() {
                tracing::debug!("restored page close failed");
            }
        }
        Ok(())
    }

    async fn bootstrap(&mut self) -> Result<(), anyhow::Error> {
        if self.launched {
            self.close_restored_pages().await?;
        }
        self.create_pages().await
    }
}
