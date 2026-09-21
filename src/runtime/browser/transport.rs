use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide::Page;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{gate::ProfileGate, BrowserError, BrowserResponse};
use crate::runtime::source::http::challenge::cf_header_challenge;
use crate::runtime::source::request::RequestSpec;
mod capture;
mod cleanup;
mod fetch_loop;
mod fetch_state;
mod rankings;
mod script;
pub(super) use capture::{capture_body, response_headers};
use fetch_loop::{drive_capture, subscribe_fetch};
use fetch_state::{complete_fetch, FetchCapture};
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
    let listeners = subscribe_fetch(page).await?;
    // Recheck gate/generation immediately after async listener setup.
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let request_body = request
        .body()
        .map(|body| serde_json::to_string(&body))
        .transpose()
        .map_err(|_| BrowserError::Protocol)?;
    let mut capture = FetchCapture::new(page, &gate, request, request_body.as_deref());
    drive_capture(&mut capture, listeners, request_timeout).await?;
    complete_fetch(capture).await
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
