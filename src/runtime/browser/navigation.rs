use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    RequestId,
};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::Page;
use futures::StreamExt;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use url::Url;

use super::{gate::ProfileGate, transport, BrowserError};
use crate::runtime::source::http::challenge::{cf_header_challenge, html_body_challenge};
use crate::runtime::source::retry::retry_after as retry_after_source;

mod observer;
pub(crate) use observer::start_observer;
use observer::{is_main_document_for, load_observation, navigation_status, Observation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NavigationOutcome {
    Ready,
    Challenged,
    CoolingDown(Duration),
    Failed(u16),
    Pending,
}

/// Bootstrap a page to the target origin. Collects events, captures body,
/// classifies outcome, and signals challenge via gate.revoke().
///
/// Does NOT call gate.try_open — that is the actor's sole responsibility.
/// Does NOT store observation — the observer handles all observation persistence.
/// Gate starts closed; bootstrap MUST run while closed.
pub(crate) async fn bootstrap(
    page: &Page,
    target: &Url,
    timeout: Duration,
    gate: Arc<ProfileGate>,
) -> Result<NavigationOutcome, BrowserError> {
    let main_frame = page
        .mainframe()
        .await
        .map_err(|_| BrowserError::Transport)?
        .ok_or(BrowserError::Unavailable)?;
    let mut requests = page
        .event_listener::<EventRequestWillBeSent>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut responses = page
        .event_listener::<EventResponseReceived>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut finished = page
        .event_listener::<EventLoadingFinished>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut failures = page
        .event_listener::<EventLoadingFailed>()
        .await
        .map_err(|_| BrowserError::Transport)?;

    let navigation = page.goto(NavigateParams::new(target.to_string()));
    tokio::pin!(navigation);
    let deadline = match Instant::now().checked_add(timeout) {
        Some(value) => value,
        None => Instant::now() + Duration::from_secs(300), // overflow guard
    };
    let mut navigation_done = false;

    let mut latest_request_id: Option<RequestId> = None;
    let mut document_id: Option<RequestId> = None;
    let mut observation = Observation {
        url: target.to_string(),
        status: None,
        headers: HeaderMap::new(),
        challenged: false,
        body_challenged: false,
        body_complete: false,
        failed: false,
        denied: false,
    };
    let mut pending_response: Option<Arc<EventResponseReceived>> = None;
    let mut pending_finished: Option<RequestId> = None;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(BrowserError::Timeout);
        }
        tokio::select! {
            biased;
            result = &mut navigation, if !navigation_done => {
                result.map_err(|_| BrowserError::Transport)?;
                navigation_done = true;
            }
            event = requests.next() => {
                let Some(event) = event else { return Err(BrowserError::Transport); };
                if is_main_document_for(&event, &main_frame) {
                    let id = event.request_id.clone();
                    latest_request_id = Some(id.clone());
                    document_id = Some(id.clone());
                    observation.url = event.request.url.clone();
                    observation.status = None;
                    observation.headers = HeaderMap::new();
                    observation.challenged = false;
                    observation.body_challenged = false;
                    observation.body_complete = false;
                    observation.failed = false;
                    pending_response.take();
                    if pending_finished.as_ref() == Some(&id) {
                        let body = transport::capture_body(page, id.clone()).await?;
                        let media_type = observation
                            .headers
                            .get(CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .map_or("", |value| value);
                        observation.body_challenged = html_body_challenge(media_type, &body);
                        observation.body_complete = true;
                    }
                }
            }
            event = responses.next() => {
                let Some(event) = event else { return Err(BrowserError::Transport); };
                if let Some(latest) = &latest_request_id {
                    if latest != &event.request_id {
                        continue;
                    }
                }
                if document_id.as_ref() == Some(&event.request_id) {
                    observation.status = Some(navigation_status(event.response.status)?);
                    observation.headers = transport::response_headers(&event)?;
                    observation.challenged = cf_header_challenge(&observation.headers);
                    if observation.challenged {
                        gate.revoke();
                    }
                } else if document_id.is_none() {
                    pending_response = Some(event);
                }
            }
            event = finished.next() => {
                let Some(event) = event else { return Err(BrowserError::Transport); };
                if let Some(latest) = &latest_request_id {
                    if latest != &event.request_id {
                        continue;
                    }
                }
                if document_id.as_ref() == Some(&event.request_id) {
                    let body = transport::capture_body(page, event.request_id.clone()).await?;
                    let media_type = observation
                        .headers
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map_or("", |value| value);
                    observation.body_challenged = html_body_challenge(media_type, &body);
                    observation.body_complete = true;
                } else if document_id.is_none() {
                    pending_finished = Some(event.request_id.clone());
                }
            }
            event = failures.next() => {
                let Some(event) = event else { return Err(BrowserError::Transport); };
                if let Some(latest) = &latest_request_id {
                    if latest != &event.request_id {
                        continue;
                    }
                }
                if document_id.as_ref() == Some(&event.request_id) {
                    observation.failed = true;
                    observation.body_complete = true;
                }
            }
            _ = tokio::time::sleep(remaining) => return Err(BrowserError::Timeout),
        }
        if navigation_done && observation.status.is_some() && observation.body_complete {
            break;
        }
    }
    classify_observation(observation, &gate)
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
    classify_observation(observation, &gate)
}

fn classify_observation(
    observation: Observation,
    gate: &ProfileGate,
) -> Result<NavigationOutcome, BrowserError> {
    // Check 429/cooldown FIRST — Retry-After header takes precedence.
    if observation.status.is_some() && observation.status.unwrap_or(0) == 429 {
        let cooldown = retry_after(&observation.headers)?;
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
            &observation.headers,
        )?));
    }
    if !(200..300).contains(&status) {
        return Ok(NavigationOutcome::Failed(status));
    }
    Ok(NavigationOutcome::Ready)
}
fn retry_after(headers: &HeaderMap) -> Result<Duration, BrowserError> {
    let delay =
        retry_after_source(headers, SystemTime::now()).map_err(|_| BrowserError::Protocol)?;
    // Absent or zero Retry-After — use conservative 60s.
    if delay.is_zero() {
        return Ok(Duration::from_secs(60));
    }
    Ok(delay)
}
