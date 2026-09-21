use super::super::MAX_CAPTURE_EVENTS;
use super::capture::{process_capture_event, CaptureContext, CaptureEvent};
use super::rankings_helper::{
    build_interceptor_script, build_response, build_ui_url, transport, BINDING_NAME,
};
use super::results::fetch_results;
use crate::runtime::browser::{gate::ProfileGate, BrowserError, BrowserResponse};
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::browser_protocol::page::{
    AddScriptToEvaluateOnNewDocumentParams, RemoveScriptToEvaluateOnNewDocumentParams,
    ScriptIdentifier,
};
use chromiumoxide::cdp::js_protocol::runtime::{
    AddBindingParams, EventBindingCalled, RemoveBindingParams,
};
use chromiumoxide::Page;
use futures::{StreamExt, TryStreamExt};
use std::sync::Arc;
use std::time::Duration;

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
    if action.capture == RankingsCapture::Results {
        return fetch_results(page, action, request_timeout, gate, source_origin).await;
    }
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
        // Default Network.enable. A durable 32 MiB event buffer makes Chromium
        // replay oversized buffered messages that the CDP client cannot parse
        // ("WS Invalid message"), which desynchronises command responses and
        // surfaces as transport failures. The ranking payload is captured by
        // the injected binding, so no buffered replay is required.
        transport(
            page.execute(chromiumoxide::cdp::browser_protocol::network::EnableParams::default())
                .await,
            "network.enable",
        )?;
        transport(
            page.execute(chromiumoxide::cdp::js_protocol::runtime::EnableParams::default())
                .await,
            "runtime.enable",
        )?;
        let installed = transport(
            page.execute(
                AddScriptToEvaluateOnNewDocumentParams::builder()
                    .source(script)
                    .build()
                    .map_err(|_| BrowserError::Protocol)?,
            )
            .await,
            "add_interceptor_script",
        )?;
        script_id = Some(installed.identifier.clone());
        binding_attempted = true;
        transport(
            page.execute(
                AddBindingParams::builder()
                    .name(BINDING_NAME)
                    .build()
                    .map_err(|_| BrowserError::Protocol)?,
            )
            .await,
            "add_binding",
        )?;
        let binding_events = transport(
            page.event_listener::<EventBindingCalled>().await,
            "binding_listener",
        )?;
        let response_events = transport(
            page.event_listener::<EventResponseReceived>().await,
            "response_listener",
        )?;
        let ui_url = build_ui_url(source_origin, action)?;
        transport(
            page.goto(chromiumoxide::cdp::browser_protocol::page::NavigateParams::new(ui_url))
                .await,
            "navigate",
        )?;
        let events = futures::stream::select(
            binding_events
                .map(CaptureEvent::Binding)
                .chain(futures::stream::once(async { CaptureEvent::Closed })),
            response_events
                .map(CaptureEvent::Response)
                .chain(futures::stream::once(async { CaptureEvent::Closed })),
        )
        .take(MAX_CAPTURE_EVENTS)
        .take_until(tokio::time::sleep_until(absolute_deadline));
        futures::pin_mut!(events);
        let capture_gate = gate.as_ref();
        let capture_origin = origin.as_str();
        let capture_context = CaptureContext {
            page,
            action,
            origin: capture_origin,
            nonce,
            deadline: absolute_deadline,
            gate: capture_gate,
        };
        let capture_context = &capture_context;
        let candidates = futures::stream::unfold(
            (events, false),
            |(mut events, mut page_one_seen)| async move {
                let event = events.next().await?;
                let candidate =
                    process_capture_event(event, capture_context, &mut page_one_seen).await;
                Some((candidate, (events, page_one_seen)))
            },
        )
        .try_filter_map(|candidate| async move { Ok(candidate) });
        futures::pin_mut!(candidates);
        let captured = candidates
            .next()
            .await
            .transpose()?
            .ok_or(BrowserError::Timeout)?;
        let response = build_response(captured)?;
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
                page.execute(chromiumoxide::cdp::browser_protocol::page::CloseParams::default()),
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
                page.execute(chromiumoxide::cdp::browser_protocol::page::CloseParams::default()),
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
    if binding_attempted
        && page
            .execute(RemoveBindingParams::new(BINDING_NAME))
            .await
            .is_err()
    {
        tracing::error!("rankings cleanup failed while removing binding");
        if cleanup_error.is_none() {
            cleanup_error = Some(BrowserError::Transport);
        }
    }
    cleanup_error.map_or(Ok(()), Err)
}
