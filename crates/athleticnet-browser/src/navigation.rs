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

/// Chromium reports `net::ERR_ABORTED` for a request the browser itself
/// superseded, which includes the original document of a redirect chain. The
/// rankings source canonicalises navigations (it strips a trailing slash and a
/// `page=1` query), so a redirect-free URL is not always available and the
/// abandoned first document MUST NOT latch a transport failure.
pub(super) const REDIRECT_ABORT: &str = "net::ERR_ABORTED";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NavigationOutcome {
    Ready,
    Challenged,
    CoolingDown(Duration),
    Failed(u16),
    Pending,
}

/// How often a settling wait re-samples a challenged tab. One sample costs two evaluations in the
/// tab, so the interval is what keeps a wait measured in tens of seconds cheap.
const CHALLENGE_POLL: Duration = Duration::from_millis(250);

/// Bootstrap a page to the target origin. Collects events, captures body,
/// classifies outcome, and signals challenge via gate.revoke().
///
/// A challenged classification is not yet a verdict: a managed interstitial usually clears itself
/// by running its platform script, which mints the clearance cookie and reloads the tab.
/// `challenge_wait` is the budget for that settlement, so what this returns is the page's answer
/// after waiting rather than the first interstitial that happened to be served.
///
/// Does NOT call gate.try_open — that is the actor's sole responsibility.
/// Does NOT store observation — the observer handles all observation persistence.
/// Gate starts closed; bootstrap MUST run while closed.
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

/// Re-sample a challenged tab until it stops reporting a challenge, or the budget runs out.
///
/// The sample is [`inspect`], which reads the document the tab is showing *now*: the observer
/// replaces a page's observation when the tab loads another document, so a challenge that its own
/// script cleared reads here as the page it became. A challenge that outlasts the budget is
/// reported as challenged, which is the latch that asks a human for the step.
///
/// The injected clock supplies the deadline and tokio's timer the waits, so a paused test drives
/// the whole budget without real time passing.
async fn settle<F, Fut>(
    clock: &dyn Clock,
    budget: Duration,
    mut sample: F,
) -> Result<NavigationOutcome, BrowserError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<NavigationOutcome, BrowserError>>,
{
    // A budget the platform clock cannot represent is no budget: the profile is reported as
    // challenged rather than pinned to a deadline that cannot exist.
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

/// Resolve the frame every captured event is judged against.
async fn resolve_main_frame(page: &Page) -> Result<FrameId, BrowserError> {
    page.mainframe()
        .await
        .map_err(|_| BrowserError::Transport)?
        .ok_or(BrowserError::Unavailable)
}

/// The half of a navigation loop that does not change while it runs.
///
/// The mutable half — the event streams and the document — stays a separate argument, so the loop
/// body reads as one `select!` over the things that move.
struct NavigationLoop<'a> {
    page: &'a Page,
    target: &'a Url,
    main_frame: &'a FrameId,
    deadline: tokio::time::Instant,
    clock: &'a dyn Clock,
    gate: &'a ProfileGate,
}

impl NavigationLoop<'_> {
    /// Drive the navigation and its event streams until the document is complete or the deadline
    /// expires.
    ///
    /// One absolute deadline governs the whole loop: every pass recomputes the remaining budget, so
    /// a stream that keeps producing events cannot extend the navigation, and the `sleep` arm is
    /// what turns the deadline into `BrowserError::Timeout`.
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

/// Inspect a page's current navigation state. Classifies outcome and
/// signals challenge via gate.revoke().
///
/// Does NOT call gate.try_open — that is the actor's sole responsibility.
/// Gate starts closed; inspect MUST run while closed.
/// If page is still loading, returns Pending so actor waits/retries.
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
    // Foreign origin: abort immediately.
    if current.origin() != origin.origin() {
        return Err(BrowserError::Unavailable);
    }
    // Same-origin but observation.url differs — pending URL transition.
    if observation.url != current_url {
        return Ok(NavigationOutcome::Pending);
    }
    // Page still loading — return Pending so actor waits/retries.
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
    // Check 429/cooldown FIRST — Retry-After header takes precedence.
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
    // body_complete must be true — response headers alone do not
    // guarantee the body has been captured.
    // Pending: observation is incomplete — missing body or headers.
    if !observation.body_complete {
        return Ok(NavigationOutcome::Pending);
    }
    // Same-origin URL transition — body captured but status unknown.
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
    // Absent or zero Retry-After — use conservative 60s.
    if delay.is_zero() {
        return Ok(Duration::from_secs(60));
    }
    Ok(delay)
}

#[cfg(test)]
mod tests;
