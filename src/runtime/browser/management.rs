use crate::runtime::browser::navigation::{self, NavigationOutcome};
use crate::runtime::browser::{
    actor::{Actor, ChallengeTarget, Command},
    pool::{self, PageSlot},
    BrowserError, BrowserResponse, BrowserState,
};
use chromiumoxide::cdp::browser_protocol::target::CreateTargetParams;
use std::time::{Duration, Instant};

impl Actor {
    pub(in crate::runtime::browser) fn complete_observer(
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
                pool::reject_pending(&mut self.pending, BrowserError::Unavailable);
            }
            None => {}
        }
    }

    pub(in crate::runtime::browser) fn latch_challenge(
        &mut self,
        request: Option<&crate::runtime::source::request::RequestSpec>,
    ) {
        if let Some(value) = request {
            self.challenge_target = Some(ChallengeTarget {
                url: value.url.clone(),
                post: value.body.is_some(),
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
        pool::reject_pending(&mut self.pending, BrowserError::HumanRequired);
    }

    pub(in crate::runtime::browser) fn apply_navigation(&mut self, outcome: NavigationOutcome) {
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

    pub(in crate::runtime::browser) fn apply_cooldown(&mut self, response: &BrowserResponse) {
        match crate::runtime::source::retry::retry_after(
            &response.headers,
            std::time::SystemTime::now(),
        ) {
            Ok(delay) => self.apply_cooldown_duration(delay),
            Err(_) => {
                let delay = match response.headers.get("Retry-After") {
                    Some(value) => value
                        .to_str()
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(30),
                    None => 30,
                };
                self.apply_cooldown_duration(Duration::from_secs(delay as u64));
            }
        }
    }

    pub(in crate::runtime::browser) fn apply_cooldown_duration(&mut self, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        let now = Instant::now();
        let until = match now.checked_add(delay) {
            Some(value) => value,
            None => now + Duration::from_secs(86_400),
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

    pub(in crate::runtime::browser) async fn command(&mut self, command: Option<Command>) {
        match command {
            Some(Command::Bootstrap { reply }) => {
                let _ = reply.send(self.bootstrap().await);
            }
            Some(Command::Fetch { request, reply }) => self.accept_fetch(request, reply),
            Some(Command::Inspect { reply }) => {
                let _ = reply.send(self.inspect_page().await);
            }
            Some(Command::Recover { reply }) => {
                let _ = reply.send(self.recover_page().await);
            }
            Some(Command::Shutdown { reply }) => {
                self.shutdown_reply = Some(reply);
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.gate.revoke();
                pool::reject_pending(&mut self.pending, BrowserError::Shutdown);
            }
            None => {
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.gate.revoke();
                pool::reject_pending(&mut self.pending, BrowserError::Shutdown);
            }
        }
    }

    pub(in crate::runtime::browser) async fn create_pages(&mut self) -> anyhow::Result<()> {
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
            self.observers.spawn(observer.run());
            let outcome = navigation::bootstrap(
                &page,
                &self.settings.source_origin,
                self.settings.request_timeout,
                self.gate.clone(),
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

    async fn bootstrap(&mut self) -> Result<(), anyhow::Error> {
        self.create_pages().await
    }
}
