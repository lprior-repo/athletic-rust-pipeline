use super::super::rankings_helper::{parse_binding, CapturedRanking, BINDING_NAME};
use super::CaptureContext;
use crate::protocol::RankingsCapture;
use crate::{gate::ProfileGate, BrowserError};
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::js_protocol::runtime::EventBindingCalled;

#[cfg(test)]
mod tests;

pub(super) fn binding_package(
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

pub(super) fn resolve_binding(
    package: &serde_json::Value,
    context: &CaptureContext<'_>,
) -> Result<Option<CapturedRanking>, BrowserError> {
    match parse_binding(package, context.action.capture.clone()) {
        Ok(value)
            if validate_route(
                &value.response_url,
                context.origin,
                context.action.capture.clone(),
                &value.method,
            ) =>
        {
            Ok(Some(value))
        }
        Ok(_) => Err(BrowserError::Redirect),
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

pub(super) fn observe_response_event(
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
    let Ok(_status) = u16::try_from(event.response.status) else {
        return;
    };
    if event
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

pub(super) fn route_path_matches(url: &str, origin: &str, kind: RankingsCapture) -> bool {
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
    parsed.path() == expected_path
}

pub(super) fn validate_route(url: &str, origin: &str, kind: RankingsCapture, method: &str) -> bool {
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
