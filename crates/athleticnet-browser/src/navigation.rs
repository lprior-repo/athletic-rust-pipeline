use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::page::{FrameId, NavigateParams};
use chromiumoxide::Page;
use futures::StreamExt;
use reqwest::header::HeaderMap;
use url::Url;

use super::{gate::ProfileGate, BrowserError};
use crate::clock::Clock;
use crate::retry::retry_after_now;

mod document;
mod observer;
use document::{bootstrap_events, navigation_deadline, BootstrapEvents, DocumentState};
pub(crate) use observer::start_observer;
use observer::{load_observation, Observation};

pub(super) const REDIRECT_ABORT: &str = "net::ERR_ABORTED";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NavigationOutcome {
    Ready,
    Challenged,
    CoolingDown(Duration),
    Failed(u16),
    Pending,
}

const CHALLENGE_POLL: Duration = Duration::from_millis(250);

pub(crate) async fn bootstrap(
    page: &Page,
    target: &Url,
    timeout: Duration,
    challenge_wait: Duration,
    gate: Arc<ProfileGate>,
    clock: &dyn Clock,
) -> Result<NavigationOutcome, BrowserError> {
    let main_frame = resolve_main_frame(page).await?;
    let mut events = bootstrap_events(page).await?;
    let loop_context = NavigationLoop {
        page,
        target,
        main_frame: &main_frame,
        deadline: navigation_deadline(clock.now_instant(), timeout),
        clock,
        gate: &gate,
    };
    let mut state = DocumentState::new(target.to_string());
    loop_context.run(&mut events, &mut state).await?;
    let outcome = classify_observation(state.into_observation(), &gate, clock)?;
    if !matches!(outcome, NavigationOutcome::Challenged) {
        return Ok(outcome);
    }
    settle(clock, challenge_wait, || {
        inspect(page, target, gate.clone(), clock)
    })
    .await
}

async fn settle<F, Fut>(
    clock: &dyn Clock,
    budget: Duration,
    mut sample: F,
) -> Result<NavigationOutcome, BrowserError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<NavigationOutcome, BrowserError>>,
{
    let Some(deadline) = clock.now_instant().checked_add(budget) else {
        return Ok(NavigationOutcome::Challenged);
    };
    loop {
        match sample().await? {
            NavigationOutcome::Challenged | NavigationOutcome::Pending => {}
            settled => return Ok(settled),
        }
        let remaining = deadline.saturating_duration_since(clock.now_instant());
        if remaining.is_zero() {
            return Ok(NavigationOutcome::Challenged);
        }
        tokio::time::sleep(CHALLENGE_POLL.min(remaining)).await;
    }
}

async fn resolve_main_frame(page: &Page) -> Result<FrameId, BrowserError> {
    page.mainframe()
        .await
        .map_err(|_| BrowserError::Transport)?
        .ok_or(BrowserError::Unavailable)
}

struct NavigationLoop<'a> {
    page: &'a Page,
    target: &'a Url,
    main_frame: &'a FrameId,
    deadline: tokio::time::Instant,
    clock: &'a dyn Clock,
    gate: &'a ProfileGate,
}

impl NavigationLoop<'_> {
    async fn run(
        &self,
        events: &mut BootstrapEvents,
        state: &mut DocumentState,
    ) -> Result<(), BrowserError> {
        let navigation = self.page.goto(NavigateParams::new(self.target.to_string()));
        tokio::pin!(navigation);
        loop {
            let remaining = self
                .deadline
                .saturating_duration_since(self.clock.now_instant());
            if remaining.is_zero() {
                return Err(BrowserError::Timeout);
            }
            tokio::select! {
                biased;
                result = &mut navigation, if !state.navigated() => {
                    result.map_err(|_| BrowserError::Transport)?;
                    state.mark_navigated();
                }
                event = events.requests.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    state.on_request(self.page, &event, self.main_frame).await?;
                }
                event = events.responses.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    state.on_response(event, self.gate)?;
                }
                event = events.finished.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    state.on_finished(self.page, event).await?;
                }
                event = events.failures.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    state.on_failure(&event);
                }
                _ = tokio::time::sleep(remaining) => return Err(BrowserError::Timeout),
            }
            if state.complete() {
                return Ok(());
            }
        }
    }
}

pub(crate) async fn inspect(
    page: &Page,
    origin: &Url,
    gate: Arc<ProfileGate>,
    clock: &dyn Clock,
) -> Result<NavigationOutcome, BrowserError> {
    let current_url = page.url().await.map_err(|_| BrowserError::Transport)?;
    let ready = page
        .evaluate("document.readyState === 'complete' && !!document.body")
        .await
        .map_err(|_| BrowserError::Transport)?
        .into_value::<bool>()
        .map_err(|_| BrowserError::Protocol)?;
    let current_url = match current_url {
        Some(url) => url,
        None => return Ok(NavigationOutcome::Pending),
    };
    let observation = match load_observation(page) {
        Ok(Some(obs)) => obs,
        Ok(None) => return Ok(NavigationOutcome::Pending),
        Err(e) => return Err(e),
    };
    let current = Url::parse(&current_url).map_err(|_| BrowserError::Protocol)?;
    if current.origin() != origin.origin() {
        return Err(BrowserError::Unavailable);
    }
    if observation.url != current_url {
        return Ok(NavigationOutcome::Pending);
    }
    if !ready {
        return Ok(NavigationOutcome::Pending);
    }
    classify_observation(observation, &gate, clock)
}

fn classify_observation(
    observation: Observation,
    gate: &ProfileGate,
    clock: &dyn Clock,
) -> Result<NavigationOutcome, BrowserError> {
    if observation.status.is_some() && observation.status.unwrap_or(0) == 429 {
        let cooldown = retry_after(clock, &observation.headers)?;
        if observation.challenged || observation.body_challenged {
            gate.revoke();
        }
        return Ok(NavigationOutcome::CoolingDown(cooldown));
    }
    if observation.challenged || observation.body_challenged {
        gate.revoke();
        return Ok(NavigationOutcome::Challenged);
    }
    if observation.failed {
        return Err(BrowserError::Transport);
    }
    if !observation.body_complete {
        return Ok(NavigationOutcome::Pending);
    }
    let Some(status) = observation.status else {
        return Ok(NavigationOutcome::Pending);
    };
    if (300..400).contains(&status) {
        return Err(BrowserError::Redirect);
    }
    if status == 503 {
        return Ok(NavigationOutcome::CoolingDown(retry_after(
            clock,
            &observation.headers,
        )?));
    }
    if !(200..300).contains(&status) {
        return Ok(NavigationOutcome::Failed(status));
    }
    Ok(NavigationOutcome::Ready)
}
fn retry_after(clock: &dyn Clock, headers: &HeaderMap) -> Result<Duration, BrowserError> {
    let delay = retry_after_now(clock, headers).map_err(|_| BrowserError::Protocol)?;
    if delay.is_zero() {
        return Ok(Duration::from_secs(60));
    }
    Ok(delay)
}

#[cfg(test)]
mod tests;
