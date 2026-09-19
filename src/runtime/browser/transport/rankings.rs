use super::super::gate::ProfileGate;
use super::{BrowserError, BrowserResponse, MAX_CAPTURE_EVENTS};
use crate::runtime::protocol::{RankingPageObservation, RankingsCapture};
use crate::runtime::source::request::{self, RankingsAction};
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
#[path = "rankings_helper.rs"]
mod rankings_helper;
use rankings_helper::{
    build_interceptor_script, build_response, build_ui_url, click_numeric_page, parse_binding,
    response_has_rows, validate_request, wait_for_active_page, BINDING_NAME,
};

/// Serve a `Results` capture through the persistent in-page fetch lane: the
/// physical request is the site's own rankings API POST, issued from the
/// bootstrapped source page, so cookies, TLS, and fingerprint stay in Chromium.
/// The semantic (UI listing) URL and the receipt identity are unchanged.
async fn fetch_results(
    page: &Page,
    action: &RankingsAction,
    request_timeout: Duration,
    gate: Arc<ProfileGate>,
    source_origin: &url::Url,
) -> Result<BrowserResponse, BrowserError> {
    let request = request::rankings_spec(source_origin, action.clone())
        .map_err(|_| BrowserError::Protocol)?;
    let body = match request.body() {
        Some(body) => serde_json::to_string(&body).map_err(|_| BrowserError::Protocol)?,
        None => return Err(BrowserError::Protocol),
    };
    let mut response = super::fetch(page, &request, request_timeout, gate).await?;
    let next_page = next_page_after(&response.body, action.page);
    response.rankings = Some(RankingPageObservation {
        capture: RankingsCapture::Results,
        request_method: "POST".to_owned(),
        request_url: request.url.as_str().to_owned(),
        request_body: Some(body),
        next_page,
    });
    Ok(response)
}

/// Pagination follows the measured row-bearing signal: a page carrying ranked
/// rows requests its successor, while an empty or unparsable page ends the
/// chain. Publication seals that terminating page into the event's page count,
/// so verification requires exactly `None` on the head page and `Some(page + 1)`
/// on every page before it.
fn next_page_after(body: &[u8], page: u32) -> Option<u32> {
    match response_has_rows(body) {
        Ok(true) => page.checked_add(1),
        // Malformed pages end pagination: strict publication parsing rejects
        // them before a checkpoint is written.
        Ok(false) | Err(_) => None,
    }
}

#[cfg(test)]
mod pagination {
    use super::next_page_after;

    #[test]
    fn a_page_with_rows_requests_its_successor() {
        let body = br#"{"groupedRankings":[[{"athleteId":1}]]}"#;
        assert_eq!(next_page_after(body, 1), Some(2));
        assert_eq!(next_page_after(body, 7), Some(8));
    }

    #[test]
    fn an_empty_page_terminates_the_chain() {
        assert_eq!(next_page_after(br#"{"groupedRankings":[]}"#, 3), None);
        assert_eq!(next_page_after(br#"{"groupedRankings":[[]]}"#, 3), None);
    }

    #[test]
    fn an_unparsable_page_terminates_rather_than_advancing() {
        assert_eq!(next_page_after(b"<html>challenge</html>", 3), None);
        assert_eq!(next_page_after(b"", 3), None);
    }
}

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
        let binding_events = page
            .event_listener::<EventBindingCalled>()
            .await
            .map_err(|_| BrowserError::Transport)?;
        let response_events = page
            .event_listener::<EventResponseReceived>()
            .await
            .map_err(|_| BrowserError::Transport)?;
        let ui_url = build_ui_url(source_origin, action)?;
        page.goto(chromiumoxide::cdp::browser_protocol::page::NavigateParams::new(ui_url))
            .await
            .map_err(|_| BrowserError::Transport)?;
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

enum CaptureEvent {
    Binding(Arc<EventBindingCalled>),
    Response(Arc<EventResponseReceived>),
    Closed,
}

struct CaptureContext<'a> {
    page: &'a Page,
    action: &'a RankingsAction,
    origin: &'a str,
    nonce: u64,
    deadline: tokio::time::Instant,
    gate: &'a ProfileGate,
}

async fn process_capture_event(
    event: CaptureEvent,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
) -> Result<Option<rankings_helper::CapturedRanking>, BrowserError> {
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
) -> Result<Option<rankings_helper::CapturedRanking>, BrowserError> {
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

/// End-to-end qualification of the persistent lane against an offline fixture.
///
/// Ignored by default: each test needs a fixture origin serving the rankings API
/// and a CDP browser. Point `ADLAW_LANE_FIXTURE` (default `http://127.0.0.1:21045/`)
/// and `ADLAW_LANE_CDP` (default `http://127.0.0.1:9223`) at those, then run
/// `cargo test --lib -- --ignored lane_smoke`.
#[cfg(test)]
mod lane_smoke {
    use super::*;
    use crate::runtime::source::request::RankingsAction;
    use chromiumoxide::{handler::HandlerConfig, Browser, Page};
    use futures::StreamExt;
    use serde_json::Value;

    const API_PATH: &str = "/api/v1/tfRankings/GetRankings";
    const MEASURED_BODY: &str = r#"{"reportType":"div","mode":"list","divListId":168416,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},"qualifyingListKey":"","version":2,"debug":""}"#;

    fn var(key: &str, fallback: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| fallback.to_owned())
    }

    fn origin() -> url::Url {
        url::Url::parse(&var("ADLAW_LANE_FIXTURE", "http://127.0.0.1:21045/"))
            .expect("fixture origin")
    }

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    async fn state(fixture: &url::Url) -> Value {
        client()
            .get(fixture.join("state").expect("state url"))
            .send()
            .await
            .expect("fixture state")
            .json()
            .await
            .expect("fixture state json")
    }

    async fn set_scenario(fixture: &url::Url, name: &str) {
        client()
            .post(fixture.join("scenario").expect("scenario url"))
            .json(&serde_json::json!({ "scenario": name }))
            .send()
            .await
            .expect("fixture scenario");
    }

    fn api_calls(value: &Value) -> Vec<Value> {
        value["requests"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|row| row["path"].as_str() == Some(API_PATH))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Connect a browser whose first page is parked on the fixture origin: the
    /// lane issues its request from that document, so the page must be there.
    async fn parked_page(fixture: &url::Url) -> (Browser, Page) {
        let cdp = var("ADLAW_LANE_CDP", "http://127.0.0.1:9223");
        let (browser, mut handler) = Browser::connect_with_config(
            cdp.as_str(),
            HandlerConfig {
                request_timeout: Duration::from_secs(20),
                ..Default::default()
            },
        )
        .await
        .expect("connect browser");
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        let page = browser
            .new_page(fixture.as_str())
            .await
            .expect("open fixture page");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        while tokio::time::Instant::now() < deadline {
            if page
                .evaluate("location.origin")
                .await
                .ok()
                .and_then(|value| value.into_value::<String>().ok())
                .is_some_and(|value| value == fixture.origin().ascii_serialization())
            {
                return (browser, page);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("fixture page never reached its origin");
    }

    fn results_action(page: u32) -> RankingsAction {
        RankingsAction {
            list_id: 168416,
            gender: "m".to_owned(),
            grade: Some(11),
            event_short: "100m".to_owned(),
            page,
            capture: RankingsCapture::Results,
        }
    }

    fn open_gate() -> Arc<ProfileGate> {
        let gate = Arc::new(ProfileGate::new());
        let generation = gate.snapshot().generation;
        assert!(
            gate.try_open(generation),
            "gate opens from a clean snapshot"
        );
        gate
    }

    #[tokio::test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    async fn results_capture_costs_one_physical_post() {
        let fixture = origin();
        set_scenario(&fixture, "normal").await;
        let (_browser, page) = parked_page(&fixture).await;
        let gate = open_gate();
        let before = api_calls(&state(&fixture).await).len();
        let response = fetch_rankings(
            &page,
            &results_action(1),
            Duration::from_secs(20),
            gate.clone(),
            &fixture,
            1,
        )
        .await
        .expect("lane response");
        let observed = api_calls(&state(&fixture).await);
        assert_eq!(observed.len() - before, 1, "exactly one physical request");
        assert_eq!(observed[observed.len() - 1]["method"], "POST");
        assert_eq!(observed[observed.len() - 1]["status"], 200);
        assert_eq!(response.status.as_u16(), 200);
        assert!(gate.is_ready(), "a served response leaves the gate open");
        let observation = response.rankings.expect("capture metadata");
        assert_eq!(observation.capture, RankingsCapture::Results);
        assert_eq!(observation.request_method, "POST");
        assert_eq!(
            observation.request_url,
            fixture
                .join(API_PATH.trim_start_matches('/'))
                .expect("api url")
                .as_str()
        );
        assert_eq!(observation.request_body.as_deref(), Some(MEASURED_BODY));
        assert_eq!(observation.next_page, Some(2));
        let envelope: Value = serde_json::from_slice(&response.body).expect("rankings envelope");
        assert_eq!(envelope["settings"]["page"], 1);
        assert!(envelope["groupedRankings"]
            .as_array()
            .is_some_and(|groups| !groups.is_empty()));
        page.close().await.expect("close fixture page");
    }

    #[tokio::test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    async fn challenge_response_revokes_the_gate_and_ends_pagination() {
        let fixture = origin();
        set_scenario(&fixture, "challenge").await;
        let (_browser, page) = parked_page(&fixture).await;
        let gate = open_gate();
        let response = fetch_rankings(
            &page,
            &results_action(1),
            Duration::from_secs(20),
            gate.clone(),
            &fixture,
            1,
        )
        .await
        .expect("lane response");
        assert_eq!(response.status.as_u16(), 403);
        assert!(!gate.is_ready(), "a challenge closes the gate");
        assert_eq!(
            response.rankings.expect("capture metadata").next_page,
            None,
            "a challenged page never advances pagination"
        );
        page.close().await.expect("close fixture page");
    }
}
