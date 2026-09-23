use super::rankings_helper::{
    click_numeric_page, parse_binding, validate_request, wait_for_active_page, CapturedRanking,
    BINDING_NAME,
};
use crate::protocol::RankingsCapture;
use crate::request::RankingsAction;
use crate::{gate::ProfileGate, BrowserError};
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::js_protocol::runtime::EventBindingCalled;
use chromiumoxide::Page;
use std::sync::Arc;

pub(super) enum CaptureEvent {
    Binding(Arc<EventBindingCalled>),
    Response(Arc<EventResponseReceived>),
    Closed,
}

pub(super) struct CaptureContext<'a> {
    pub(super) page: &'a Page,
    pub(super) action: &'a RankingsAction,
    pub(super) origin: &'a str,
    pub(super) nonce: u64,
    pub(super) deadline: tokio::time::Instant,
    pub(super) gate: &'a ProfileGate,
}

pub(super) async fn process_capture_event(
    event: CaptureEvent,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
) -> Result<Option<CapturedRanking>, BrowserError> {
    match event {
        CaptureEvent::Binding(binding) => {
            process_binding_event(&binding, context, page_one_seen).await
        }
        CaptureEvent::Response(response) => {
            observe_response_event(
                &response,
                context.action.capture.clone(),
                context.origin,
                context.gate,
            );
            Ok(None)
        }
        CaptureEvent::Closed => Err(BrowserError::Transport),
    }
}

async fn process_binding_event(
    event: &EventBindingCalled,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
) -> Result<Option<CapturedRanking>, BrowserError> {
    let Some(package) = binding_package(event, context) else {
        return Ok(None);
    };
    let Some(parsed) = resolve_binding(&package, context)? else {
        return Ok(None);
    };
    if parsed.challenge {
        context.gate.revoke();
    }
    if context.action.capture == RankingsCapture::Navigation {
        return Ok(parsed.request_body.is_none().then_some(parsed));
    }
    let Some(request_page) = validate_request(
        &parsed,
        context.action,
        context.action.page > 1 && !*page_one_seen,
    )?
    else {
        return Ok(None);
    };
    if request_page == context.action.page {
        return Ok(Some(parsed));
    }
    click_through_first_page(context, request_page, page_one_seen).await?;
    Ok(None)
}

/// Decode one binding payload and check it is ours: nonce, kind and route.
fn binding_package(
    event: &EventBindingCalled,
    context: &CaptureContext<'_>,
) -> Option<serde_json::Value> {
    if event.name != BINDING_NAME {
        return None;
    }
    let package: serde_json::Value = serde_json::from_str(&event.payload).ok()?;
    let expected_kind = match context.action.capture {
        RankingsCapture::Navigation => "navigation",
        RankingsCapture::Results => "results",
    };
    if package.get("nonce").and_then(serde_json::Value::as_u64) != Some(context.nonce)
        || package.get("kind").and_then(serde_json::Value::as_str) != Some(expected_kind)
    {
        return None;
    }
    let route_url = package
        .get("requestUrl")
        .and_then(serde_json::Value::as_str);
    let route_method = package.get("method").and_then(serde_json::Value::as_str);
    if !route_url.is_some_and(|url| {
        route_method.is_some_and(|method| {
            validate_route(url, context.origin, context.action.capture.clone(), method)
        })
    }) {
        return None;
    }
    Some(package)
}

/// Parse a routed payload, revoking the gate for the errors that carry evidence.
///
/// `None` means "this payload is not a rankings capture", not a failure.
fn resolve_binding(
    package: &serde_json::Value,
    context: &CaptureContext<'_>,
) -> Result<Option<CapturedRanking>, BrowserError> {
    match parse_binding(package, context.action.capture.clone()) {
        Ok(value) => Ok(Some(value)),
        Err(BrowserError::PayloadLimit) => {
            context.gate.revoke();
            Err(BrowserError::PayloadLimit)
        }
        Err(BrowserError::Unavailable) => {
            context.gate.revoke();
            Err(BrowserError::Unavailable)
        }
        Err(BrowserError::Redirect) => Err(BrowserError::Redirect),
        Err(BrowserError::Shutdown) => Err(BrowserError::Shutdown),
        Err(BrowserError::TaskPanicked) => Err(BrowserError::TaskPanicked),
        Err(BrowserError::HumanRequired) => Err(BrowserError::HumanRequired),
        Err(BrowserError::Timeout | BrowserError::Transport | BrowserError::Protocol) => Ok(None),
    }
}

/// The requested page is behind page one — click through to it once.
///
/// Page one is only consumed when it is one ahead of the requested page and no
/// previous payload has already spent the flag.
async fn click_through_first_page(
    context: &CaptureContext<'_>,
    request_page: u32,
    page_one_seen: &mut bool,
) -> Result<(), BrowserError> {
    if *page_one_seen || request_page != 1 {
        return Ok(());
    }
    *page_one_seen = true;
    wait_for_active_page(context.page, 1, context.deadline).await?;
    if !click_numeric_page(context.page, context.action.page).await? {
        return Err(BrowserError::Timeout);
    }
    Ok(())
}

fn observe_response_event(
    event: &EventResponseReceived,
    capture: RankingsCapture,
    origin: &str,
    gate: &ProfileGate,
) {
    let expected_path = match capture {
        RankingsCapture::Navigation => "/api/v1/tfRankings/GetNavInfo",
        RankingsCapture::Results => "/api/v1/tfRankings/GetRankings",
    };
    let parsed = url::Url::parse(&event.response.url).ok();
    if !parsed.as_ref().is_some_and(|value| {
        value.origin().ascii_serialization() == origin && value.path() == expected_path
    }) {
        return;
    }
    let Ok(status) = u16::try_from(event.response.status) else {
        return;
    };
    if status == 403
        || status == 429
        || event
            .response
            .headers
            .inner()
            .get("cf-mitigated")
            .and_then(|value| value.as_str())
            == Some("challenge")
    {
        gate.revoke();
    }
}
/// Validate route: origin, API path, and method (GET=NavInfo, POST=GetRankings).
fn validate_route(url: &str, origin: &str, kind: RankingsCapture, method: &str) -> bool {
    let parsed = match url::Url::parse(url) {
        Ok(value) => value,
        Err(_) => return false,
    };
    if parsed.origin().ascii_serialization() != origin {
        return false;
    }
    let expected_path = match kind {
        RankingsCapture::Navigation => "/api/v1/tfRankings/GetNavInfo",
        RankingsCapture::Results => "/api/v1/tfRankings/GetRankings",
    };
    if parsed.path() != expected_path {
        return false;
    }
    match kind {
        RankingsCapture::Navigation => method == "GET",
        RankingsCapture::Results => method == "POST",
    }
}
