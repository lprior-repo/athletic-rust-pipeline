use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide::cdp::IntoEventKind;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{gate::ProfileGate, BrowserError, BrowserResponse};
use crate::challenge::cf_header_challenge;
use crate::clock::Clock;
use crate::request::RequestSpec;
mod capture;
mod cleanup;
mod fetch_loop;
mod fetch_state;
mod invocation;
pub(crate) mod pending;
pub(crate) mod progress;
mod rankings;
mod request_binding;
mod script;
pub(crate) mod transfer;
pub(super) use capture::{capture_body, response_headers};
use fetch_loop::{drive_capture, subscribe_fetch};
use fetch_state::{complete_fetch, FetchCapture};
pub(super) use rankings::fetch_rankings;

#[cfg(test)]
mod tests;

pub(crate) async fn subscribe<T: IntoEventKind>(
    page: &Page,
) -> Result<EventStream<T>, BrowserError> {
    page.event_listener::<T>()
        .await
        .map_err(|_| BrowserError::Transport)
}

async fn fail(
    page: &Page,
    gate: &ProfileGate,
    invocation: Option<&invocation::Invocation>,
    original: BrowserError,
) -> BrowserError {
    let result = cleanup::cleanup(page, gate, invocation).await;
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
    headers: &'a [(String, String)],
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FetchResult {
    ok: bool,
    error: Option<String>,
    status: Option<u16>,
}

impl FetchResult {
    fn validate(&self) -> Result<(), BrowserError> {
        match (self.ok, self.error.as_deref(), self.status) {
            (true, None, Some(_)) => Ok(()),
            (false, Some("payload_limit"), _) => Err(BrowserError::PayloadLimit),
            (false, Some("aborted"), _) => Err(BrowserError::Timeout),
            (false, Some("network"), _) => Err(BrowserError::Transport),
            _ => Err(BrowserError::Protocol),
        }
    }
}
#[derive(Debug, Clone)]
struct ResponseEvidence {
    status: u16,
    response_url: String,
    headers: HeaderMap,
}

pub(crate) async fn fetch(
    page: &Page,
    request: &RequestSpec,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
    clock: &dyn Clock,
) -> Result<BrowserResponse, BrowserError> {
    fetch_loop::timeout_millis(request_timeout)?;
    fetch_loop::validate_request_headers(&request.headers)?;
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let deadline = clock
        .now_instant()
        .checked_add(request_timeout)
        .ok_or(BrowserError::Protocol)?;
    let mut invocation = None;
    let attempt = tokio::time::timeout_at(
        deadline,
        capture_attempt(page, request, deadline, &gate, clock, &mut invocation),
    )
    .await;
    let error = match attempt {
        Ok(Ok(response)) => return Ok(response),
        Ok(Err(error)) => error,
        Err(_) => BrowserError::Timeout,
    };
    Err(fail(page, &gate, invocation.as_ref(), error).await)
}

async fn capture_attempt(
    page: &Page,
    request: &RequestSpec,
    deadline: tokio::time::Instant,
    gate: &ProfileGate,
    clock: &dyn Clock,
    invocation_slot: &mut Option<invocation::Invocation>,
) -> Result<BrowserResponse, BrowserError> {
    let listeners = subscribe_fetch(page).await?;
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let request_body = request
        .body()
        .map(|body| serde_json::to_string(&body))
        .transpose()
        .map_err(|_| BrowserError::Protocol)?;
    let remaining = deadline.saturating_duration_since(clock.now_instant());
    let arguments = FetchArguments {
        url: request.url.as_str(),
        method: request_method(request),
        body: request_body.as_deref(),
        timeout_ms: fetch_loop::timeout_millis(remaining)?,
        max_body: crate::protocol::MAX_SOURCE_RESPONSE_BYTES,
        headers: request.headers.as_slice(),
    };
    *invocation_slot = Some(invocation::prepare(page, &arguments).await?);
    let invocation = invocation_slot.as_ref().ok_or(BrowserError::Protocol)?;
    let mut capture = FetchCapture::new(page, gate, request, request_body.as_deref(), invocation);
    drive_capture(&mut capture, listeners, deadline, clock).await?;
    let response = complete_fetch(capture).await?;
    invocation::release(page, invocation).await?;
    Ok(response)
}

fn request_method(request: &RequestSpec) -> &'static str {
    if request.body().is_some() {
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
    let headers = response_headers(event)?;
    if cf_header_challenge(&headers) {
        *challenge_seen = true;
        gate.revoke();
    }
    *response = Some(ResponseEvidence {
        status,
        response_url: event.response.url.clone(),
        headers,
    });
    Ok(())
}
