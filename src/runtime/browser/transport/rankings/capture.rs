use super::rankings_helper::{
    click_numeric_page, parse_binding, validate_request, wait_for_active_page, CapturedRanking,
    BINDING_NAME,
};
use crate::runtime::browser::{gate::ProfileGate, BrowserError};
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
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
    if event.name != BINDING_NAME {
        return Ok(None);
    }
    let package: serde_json::Value = match serde_json::from_str(&event.payload) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let expected_kind = match context.action.capture {
        RankingsCapture::Navigation => "navigation",
        RankingsCapture::Results => "results",
    };
    if package.get("nonce").and_then(serde_json::Value::as_u64) != Some(context.nonce)
        || package.get("kind").and_then(serde_json::Value::as_str) != Some(expected_kind)
    {
        return Ok(None);
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
        return Ok(None);
    }
    let parsed = match parse_binding(&package, context.action.capture.clone()) {
        Ok(value) => value,
        Err(BrowserError::PayloadLimit) => {
            context.gate.revoke();
            return Err(BrowserError::PayloadLimit);
        }
        Err(BrowserError::Unavailable) => {
            context.gate.revoke();
            return Err(BrowserError::Unavailable);
        }
        Err(BrowserError::Redirect) => return Err(BrowserError::Redirect),
        Err(BrowserError::Shutdown) => return Err(BrowserError::Shutdown),
        Err(BrowserError::TaskPanicked) => return Err(BrowserError::TaskPanicked),
        Err(BrowserError::HumanRequired) => return Err(BrowserError::HumanRequired),
        Err(BrowserError::Timeout | BrowserError::Transport | BrowserError::Protocol) => {
            return Ok(None);
        }
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
    if !*page_one_seen && request_page == 1 {
        *page_one_seen = true;
        wait_for_active_page(context.page, 1, context.deadline).await?;
        if !click_numeric_page(context.page, context.action.page).await? {
            return Err(BrowserError::Timeout);
        }
    }
    Ok(None)
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
