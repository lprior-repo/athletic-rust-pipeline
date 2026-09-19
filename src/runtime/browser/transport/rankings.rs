use std::sync::Arc;
use std::time::Duration;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::browser_protocol::page::{
    AddScriptToEvaluateOnNewDocumentParams, RemoveScriptToEvaluateOnNewDocumentParams,
    ScriptIdentifier,
};
use chromiumoxide::cdp::js_protocol::runtime::{
    AddBindingParams, EventBindingCalled, RemoveBindingParams,
};
use chromiumoxide::Page;
use futures::StreamExt;
use super::{BrowserError, BrowserResponse};
use super::super::gate::ProfileGate;
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
#[path = "rankings_helper.rs"]
mod rankings_helper;
use rankings_helper::{
    build_interceptor_script, build_response, click_numeric_page, extract_next_page,
    parse_binding, request_page, response_has_rows, validate_request_scope, wait_for_active_page,
    BINDING_NAME,
};

/// Fetch rankings data from the target source via browser CDP.
///
/// All setup, navigation, and capture awaits are bounded by a single
/// absolute `tokio::time::Instant` deadline.  Cleanup runs on every
/// exit path (success, timeout, transport error) and is itself bounded
pub(crate) async fn fetch_rankings(
    page: &Page,
    action: &RankingsAction,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
    source_origin: &url::Url,
    nonce: u64,
) -> Result<BrowserResponse, BrowserError> {
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let absolute_deadline = tokio::time::Instant::now()
        .checked_add(request_timeout)
        .ok_or(BrowserError::Protocol)?;
    let origin = source_origin.origin().ascii_serialization();
    let script = build_interceptor_script(&origin, nonce).map_err(|_| BrowserError::Protocol)?;
    let mut script_id: Option<ScriptIdentifier> = None;
    let mut binding_attempted = false;
    let result = tokio::time::timeout_at(absolute_deadline, async {
        page.execute(
            chromiumoxide::cdp::browser_protocol::network::EnableParams::builder()
                .max_total_buffer_size(33_554_432)
                .max_resource_buffer_size(8_388_608)
                .enable_durable_messages(true)
                .build(),
        )
        .await
        .map_err(|_| BrowserError::Transport)?;
        page.execute(chromiumoxide::cdp::js_protocol::runtime::EnableParams::default())
            .await
            .map_err(|_| BrowserError::Transport)?;
        let installed = page
            .execute(
                AddScriptToEvaluateOnNewDocumentParams::builder()
                    .source(script)
                    .build()
                    .map_err(|_| BrowserError::Protocol)?,
            )
            .await
            .map_err(|_| BrowserError::Transport)?;
        script_id = Some(installed.identifier.clone());
        binding_attempted = true;
        page.execute(
            AddBindingParams::builder()
                .name(BINDING_NAME)
                .build()
                .map_err(|_| BrowserError::Protocol)?,
        )
        .await
        .map_err(|_| BrowserError::Transport)?;
        let mut binding_events = page
            .event_listener::<EventBindingCalled>()
            .await
            .map_err(|_| BrowserError::Transport)?;
        let mut response_events = page
            .event_listener::<EventResponseReceived>()
            .await
            .map_err(|_| BrowserError::Transport)?;
        let ui_url = build_ui_url(source_origin, action)?;
        page.goto(chromiumoxide::cdp::browser_protocol::page::NavigateParams::new(ui_url))
            .await
            .map_err(|_| BrowserError::Transport)?;
        let mut page_one_seen = false;
        let captured = loop {
            let candidate = loop {
                let remaining =
                    absolute_deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break None;
                }
                tokio::select! {
                    biased;
                    event = binding_events.next() => {
                        let Some(event) = event else { break None; };
                        if event.name != BINDING_NAME { continue; }
                        let package: serde_json::Value = match serde_json::from_str(&event.payload) {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        if package.get("nonce").and_then(serde_json::Value::as_u64) != Some(nonce) {
                            continue;
                        }
                        let expected_kind = match action.capture {
                            RankingsCapture::Navigation => "navigation",
                            RankingsCapture::Results => "results",
                        };
                        if package.get("kind").and_then(serde_json::Value::as_str)
                            != Some(expected_kind)
                        {
                            continue;
                        }
                        let route_url = package.get("requestUrl").and_then(serde_json::Value::as_str);
                        let route_method = package.get("method").and_then(serde_json::Value::as_str);
                        if !route_url.is_some_and(|url| {
                            route_method.is_some_and(|method| {
                                validate_route(url, &origin, action.capture.clone(), method)
                            })
                        }) {
                            continue;
                        }
                        let parsed = match parse_binding(&package, action.capture.clone()) {
                            Ok(value) => value,
                            Err(BrowserError::PayloadLimit) => {
                                gate.revoke();
                                return Err(BrowserError::PayloadLimit);
                            }
                            Err(BrowserError::Unavailable) => {
                                gate.revoke();
                                return Err(BrowserError::Unavailable);
                            }
                            Err(BrowserError::Redirect) => return Err(BrowserError::Redirect),
                            Err(BrowserError::Shutdown) => return Err(BrowserError::Shutdown),
                            Err(BrowserError::HumanRequired) => {
                                return Err(BrowserError::HumanRequired);
                            }
                            Err(BrowserError::Timeout)
                            | Err(BrowserError::Transport)
                            | Err(BrowserError::Protocol) => continue,
                        };
                        if !validate_request_scope(
                            &parsed,
                            action,
                            action.page > 1 && !page_one_seen,
                        )? {
                            continue;
                        }
                        if parsed.challenge {
                            gate.revoke();
                        }
                        let request_page = request_page(&parsed)?;
                        if action.capture == RankingsCapture::Navigation
                            || request_page == Some(action.page)
                        {
                            break Some(parsed);
                        }
                        if action.capture == RankingsCapture::Results
                            && action.page > 1
                            && !page_one_seen
                            && request_page == Some(1)
                        {
                            page_one_seen = true;
                            let active = wait_for_active_page(page, absolute_deadline).await?;
                            if active == 1 {
                                if !click_numeric_page(page, action.page).await? {
                                    return Err(BrowserError::Timeout);
                                }
                            } else if active != action.page {
                                return Err(BrowserError::Transport);
                            }
                        }
                    }
                    event = response_events.next() => {
                        let Some(event) = event else { break None; };
                        let expected_path = match action.capture {
                            RankingsCapture::Navigation => "/api/v1/tfRankings/GetNavInfo",
                            RankingsCapture::Results => "/api/v1/tfRankings/GetRankings",
                        };
                        let parsed = url::Url::parse(&event.response.url).ok();
                        if parsed.as_ref().is_some_and(|value| {
                            value.origin().ascii_serialization() == origin
                                && value.path() == expected_path
                        }) {
                            let status = match u16::try_from(event.response.status) {
                                Ok(status) => status,
                                Err(_) => continue,
                            };
                            if status == 403 || status == 429 {
                                gate.revoke();
                            }
                            if event.response.headers.inner().get("cf-mitigated")
                                .and_then(|value| value.as_str()) == Some("challenge")
                            {
                                gate.revoke();
                            }
                        }
                    }
                    _ = tokio::time::sleep(remaining) => break None,
                }
            };
            if let Some(value) = candidate {
                break value;
            }
            if tokio::time::Instant::now() >= absolute_deadline {
                return Err(BrowserError::Timeout);
            }
        };
        let mut response = build_response(captured)?;
        if action.capture == RankingsCapture::Results && response.status.is_success() {
            let has_rows = response_has_rows(&response.body)?;
            let next_page =
                extract_next_page(page, action.page, absolute_deadline, has_rows).await?;
            if let Some(observation) = response.rankings.as_mut() {
                observation.next_page = next_page;
            }
        }
        Ok::<_, BrowserError>(response)
    })
    .await;
    let cleanup = tokio::time::timeout(
        Duration::from_secs(5),
        cleanup_capture(page, script_id.as_ref(), binding_attempted),
    )
    .await;
    match cleanup {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            gate.revoke();
            tokio::time::timeout(
                Duration::from_secs(5),
                page.execute(
                    chromiumoxide::cdp::browser_protocol::page::CloseParams::default(),
                ),
            )
            .await
            .map_err(|_| BrowserError::Timeout)?
            .map_err(|_| BrowserError::Transport)?;
            return Err(error);
        }
        Err(_) => {
            gate.revoke();
            tokio::time::timeout(
                Duration::from_secs(5),
                page.execute(
                    chromiumoxide::cdp::browser_protocol::page::CloseParams::default(),
                ),
            )
            .await
            .map_err(|_| BrowserError::Timeout)?
            .map_err(|_| BrowserError::Transport)?;
            return Err(BrowserError::Timeout);
        }
    }
    match result {
        Ok(inner) => inner,
        Err(_) => Err(BrowserError::Timeout),
    }
}

async fn cleanup_capture(
    page: &Page,
    script_id: Option<&ScriptIdentifier>,
    binding_attempted: bool,
) -> Result<(), BrowserError> {
    let mut cleanup_error = None;
    if page
        .evaluate(
            "if (typeof window.__RANKINGS_ORIGINAL_FETCH === 'function') { window.fetch = window.__RANKINGS_ORIGINAL_FETCH; } delete window.__RANKINGS_ORIGINAL_FETCH; delete window.retainRankingResponse;",
        )
        .await
        .is_err()
    {
        tracing::error!("rankings cleanup failed while restoring fetch");
        cleanup_error = Some(BrowserError::Transport);
    }
    if let Some(identifier) = script_id {
        let params = RemoveScriptToEvaluateOnNewDocumentParams::builder()
            .identifier(identifier.clone())
            .build()
            .map_err(|_| BrowserError::Protocol);
        match params {
            Ok(params) => {
                if page.execute(params).await.is_err() {
                    tracing::error!("rankings cleanup failed while removing script");
                    if cleanup_error.is_none() {
                        cleanup_error = Some(BrowserError::Transport);
                    }
                }
            }
            Err(error) => {
                tracing::error!("rankings cleanup failed while building script removal");
                if cleanup_error.is_none() {
                    cleanup_error = Some(error);
                }
            }
        }
    }
    if binding_attempted && page.execute(RemoveBindingParams::new(BINDING_NAME)).await.is_err() {
        tracing::error!("rankings cleanup failed while removing binding");
        if cleanup_error.is_none() {
            cleanup_error = Some(BrowserError::Transport);
        }
    }
    cleanup_error.map_or(Ok(()), Err)
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
