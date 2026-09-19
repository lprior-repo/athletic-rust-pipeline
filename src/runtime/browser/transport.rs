use std::sync::Arc;
use std::time::{Duration, Instant};

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    EventResponseReceivedExtraInfo, RequestId,
};
use chromiumoxide::Page;
use futures::StreamExt;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::warn;

use super::{gate::ProfileGate, BrowserError, BrowserResponse};
use crate::runtime::protocol::MAX_SOURCE_RESPONSE_BYTES;
use crate::runtime::source::http::challenge::{cf_header_challenge, html_body_challenge};
use crate::runtime::source::request::RequestSpec;
mod capture;
mod cleanup;
mod script;
mod rankings;
pub(super) use capture::{capture_body, response_headers};
pub(super) use rankings::fetch_rankings;

async fn fail(page: &Page, gate: &ProfileGate, original: BrowserError) -> BrowserError {
    let result = cleanup::cleanup(page, gate).await;
    if matches!(result, cleanup::CleanupResult::Unconfirmed) {
        warn!("cleanup unconfirmed — gate already revoked");
    }
    original
}

const MAX_CAPTURE_EVENTS: usize = 16_384;
#[derive(Debug, Serialize)]
struct FetchArguments<'a> {
    url: &'a str,
    method: &'static str,
    body: Option<&'a str>,
    timeout_ms: u64,
    max_body: usize,
}
#[derive(Debug, Deserialize)]
struct FetchResult {
    ok: bool,
    error: Option<String>,
    status: Option<u16>,
    body_base64: Option<String>,
}
#[derive(Debug, Clone)]
struct ResponseEvidence {
    status: u16,
    headers: HeaderMap,
}

/// Fetch a request through the browser. Uses absolute deadline so inner
/// awaits cannot exceed the budget. Does NOT call gate.try_open.
/// On header challenge: sets challenge flag and revokes gate but continues
/// bounded body capture — never aborts solely because of challenge.
pub(crate) async fn fetch(
    page: &Page,
    request: &RequestSpec,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
) -> Result<BrowserResponse, BrowserError> {
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let mut request_events = page
        .event_listener::<EventRequestWillBeSent>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut response_events = page
        .event_listener::<EventResponseReceived>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut finished_events = page
        .event_listener::<EventLoadingFinished>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let mut failed_events = page
        .event_listener::<EventLoadingFailed>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    // Subscribe to extra-info BEFORE dispatch — needed for redirect detection.
    let mut extra_events = page
        .event_listener::<EventResponseReceivedExtraInfo>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    // Recheck gate/generation immediately after async listener setup.
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let request_body = request
        .body
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| BrowserError::Protocol)?;
    let arguments = FetchArguments {
        url: request.url.as_str(),
        method: request_method(request),
        body: request_body.as_deref(),
        timeout_ms: u64::try_from(request_timeout.as_millis())
            .map_err(|_| BrowserError::Protocol)?,
        max_body: MAX_SOURCE_RESPONSE_BYTES,
    };
    let value = serde_json::to_value(arguments).map_err(|_| BrowserError::Protocol)?;
    let call = chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams::builder()
        .function_declaration(script::FETCH_FUNCTION)
        .argument(
            chromiumoxide::cdp::js_protocol::runtime::CallArgument::builder()
                .value(value)
                .build(),
        )
        .return_by_value(true)
        .await_promise(true)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let evaluation = page.evaluate_function(call);
    tokio::pin!(evaluation);
    let deadline = Instant::now() + request_timeout;
    let mut request_id: Option<RequestId> = None;
    let mut pending_response: Option<Arc<EventResponseReceived>> = None;
    let mut pending_finished: Option<RequestId> = None;
    let mut response: Option<ResponseEvidence> = None;
    let mut loading_finished = false;
    let mut evaluation_result: Option<FetchResult> = None;
    let mut challenge_seen = false;
    // Redirect status seen via extra-info for confirmed request.
    let mut redirect_status: Option<u16> = None;

    for _ in 0..MAX_CAPTURE_EVENTS {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(fail(page, &gate, BrowserError::Timeout).await);
        }
        tokio::select! {
            biased;
            result = &mut evaluation, if evaluation_result.is_none() => {
                let result = match result {
                    Ok(r) => r,
                    Err(_) => return Err(fail(page, &gate, BrowserError::Transport).await),
                };
                evaluation_result = Some(match result.into_value() {
                    Ok(r) => r,
                    Err(_) => return Err(fail(page, &gate, BrowserError::Protocol).await),
                });
            }
            event = request_events.next() => {
                let Some(event) = event else { return Err(fail(page, &gate, BrowserError::Transport).await); };
                if !matches_request(&event, request) { continue; }
                let id = event.request_id.clone();
                if request_id.as_ref().is_some_and(|current_id| current_id != &id) { continue; }
                let confirm = timeout(remaining, capture::matches_request_body(page, &event, request_body.as_deref())).await;
                match confirm {
                    Ok(Ok(true)) => {}
                    Ok(Ok(false)) => continue,
                    Ok(Err(e)) => return Err(fail(page, &gate, e).await),
                    Err(_) => return Err(fail(page, &gate, BrowserError::Timeout).await),
                }
                if event.redirect_response.is_some() { return Err(fail(page, &gate, BrowserError::Redirect).await); }
                if request_id.as_ref().is_some_and(|p| p != &id) { return Err(fail(page, &gate, BrowserError::Protocol).await); }
                request_id = Some(id.clone());
                if let Some(pending) = pending_response.take() {
                    if pending.request_id == id {
                        if let Err(e) = record_response(&pending, &mut response, &mut challenge_seen, &gate) {
                            return Err(fail(page, &gate, e).await);
                        }
                    }
                }
                if pending_finished.as_ref() == Some(&id) { loading_finished = true; }
            }
            event = response_events.next() => {
                let Some(event) = event else { return Err(fail(page, &gate, BrowserError::Transport).await); };
                if request_id.as_ref().is_some_and(|current_id| current_id != &event.request_id) { continue; }
                if request_id.as_ref() == Some(&event.request_id) {
                    if let Err(e) = record_response(&event, &mut response, &mut challenge_seen, &gate) {
                        return Err(fail(page, &gate, e).await);
                    }
                } else if request_id.is_none() { pending_response = Some(event); }
            }
            event = extra_events.next() => {
                let Some(event) = event else { return Err(fail(page, &gate, BrowserError::Transport).await); };
                // Correlate with confirmed request — redirect status only matters for it.
                if let Some(current_id) = &request_id {
                    if current_id == &event.request_id && (300..400).contains(&event.status_code) {
                        redirect_status = Some(event.status_code as u16);
                    }
                }
            }
            event = finished_events.next() => {
                let Some(event) = event else { return Err(fail(page, &gate, BrowserError::Transport).await); };
                if request_id.as_ref().is_some_and(|current_id| current_id != &event.request_id) { continue; }
                if request_id.as_ref() == Some(&event.request_id) { loading_finished = true; }
                else if request_id.is_none() { pending_finished = Some(event.request_id.clone()); }
            }
            event = failed_events.next() => {
                let Some(event) = event else { return Err(fail(page, &gate, BrowserError::Transport).await); };
                if request_id.as_ref() == Some(&event.request_id) {
                    // Redirect rejection: extra-info recorded 3xx, then loadingFailed.
                    if redirect_status.take().is_some() {
                        return Err(fail(page, &gate, BrowserError::Redirect).await);
                    }
                    return Err(fail(page, &gate, BrowserError::Transport).await);
                }
            }
            _ = tokio::time::sleep(remaining) => return Err(fail(page, &gate, BrowserError::Timeout).await),
        }
        if let Some(r) = evaluation_result.as_ref() {
            if !r.ok && r.error.as_deref() == Some("payload_limit") {
                return Err(BrowserError::PayloadLimit);
            }
        }
        if evaluation_result.is_some() && response.is_some() && loading_finished {
            break;
        }
        // If JS failed and loading finished, exit loop to return Transport.
        if evaluation_result.as_ref().is_some_and(|r| !r.ok) && loading_finished {
            break;
        }
    }
    if !loading_finished {
        return Err(fail(page, &gate, BrowserError::Timeout).await);
    }
    let evidence = response.ok_or(BrowserError::Transport)?;
    let result = evaluation_result.ok_or(BrowserError::Transport)?;
    if result.status != Some(evidence.status) {
        return Err(BrowserError::Protocol);
    }
    let encoded = result.body_base64.ok_or(BrowserError::Protocol)?;
    let body = capture::decode_body(&encoded)?;
    let body_challenge = html_body_challenge(
        evidence
            .headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map_or("", |v| v),
        &body,
    );
    if challenge_seen || body_challenge {
        gate.revoke();
    }
    Ok(BrowserResponse {
        status: reqwest::StatusCode::from_u16(evidence.status)
            .map_err(|_| BrowserError::Protocol)?,
        headers: evidence.headers,
        body,
        rankings: None,
    })
}

fn request_method(request: &RequestSpec) -> &'static str {
    if request.body.is_some() {
        "POST"
    } else {
        "GET"
    }
}

fn matches_request(event: &EventRequestWillBeSent, request: &RequestSpec) -> bool {
    event.request.url == request.url.as_str()
        && event
            .request
            .method
            .eq_ignore_ascii_case(request_method(request))
}

fn record_response(
    event: &EventResponseReceived,
    response: &mut Option<ResponseEvidence>,
    challenge_seen: &mut bool,
    gate: &ProfileGate,
) -> Result<(), BrowserError> {
    let status = event
        .response
        .status
        .try_into()
        .map_err(|_| BrowserError::Protocol)?;
    if (300..400).contains(&status) {
        return Err(BrowserError::Redirect);
    }
    if status == 403 || status == 429 {
        gate.revoke();
    }
    let headers = response_headers(event)?;
    if cf_header_challenge(&headers) {
        *challenge_seen = true;
        gate.revoke();
    }
    *response = Some(ResponseEvidence { status, headers });
    Ok(())
}
