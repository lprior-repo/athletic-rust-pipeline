use super::super::MAX_CAPTURE_EVENTS;
use super::capture::{process_capture_event, CaptureContext, CaptureEvent};
use super::rankings_helper::{
    build_interceptor_script, build_response, build_ui_url, transport, BINDING_NAME,
};
use super::results::fetch_results;
use super::teardown::shutdown_capture;
use crate::clock::Clock;
use crate::protocol::RankingsCapture;
use crate::request::RankingsAction;
use crate::{gate::ProfileGate, BrowserError, BrowserResponse};
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::browser_protocol::page::{
    AddScriptToEvaluateOnNewDocumentParams, NavigateParams, ScriptIdentifier,
};
use chromiumoxide::cdp::js_protocol::runtime::{AddBindingParams, EventBindingCalled};
use chromiumoxide::listeners::EventStream;
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
    clock: &dyn Clock,
) -> Result<BrowserResponse, BrowserError> {
    if action.capture == RankingsCapture::Results {
        return fetch_results(page, action, request_timeout, gate, source_origin, clock).await;
    }
    let snap = gate.snapshot();
    if !snap.ready {
        return Err(BrowserError::HumanRequired);
    }
    let absolute_deadline = clock
        .now_instant()
        .checked_add(request_timeout)
        .ok_or(BrowserError::Protocol)?;
    let origin = source_origin.origin().ascii_serialization();
    let script = build_interceptor_script(&origin, nonce).map_err(|_| BrowserError::Protocol)?;
    let mut attempt = CaptureRun {
        page,
        action,
        source_origin,
        gate: &gate,
        nonce,
        origin: &origin,
        script,
        deadline: absolute_deadline,
        script_id: None,
        binding_attempted: false,
    };
    let result = tokio::time::timeout_at(absolute_deadline, attempt.run()).await;
    shutdown_capture(
        page,
        &gate,
        attempt.script_id.as_ref(),
        attempt.binding_attempted,
    )
    .await?;
    match result {
        Ok(inner) => inner,
        Err(_) => Err(BrowserError::Timeout),
    }
}

/// One rankings capture attempt: the wired page plus the setup handles that
/// cleanup has to remove afterwards.
struct CaptureRun<'a> {
    page: &'a Page,
    action: &'a RankingsAction,
    source_origin: &'a url::Url,
    gate: &'a ProfileGate,
    nonce: u64,
    origin: &'a str,
    script: String,
    deadline: tokio::time::Instant,
    script_id: Option<ScriptIdentifier>,
    binding_attempted: bool,
}

impl CaptureRun<'_> {
    /// Wire the page, navigate to the rankings UI, and stream events until one
    /// rankings response is captured. The caller's deadline cancels this future.
    async fn run(&mut self) -> Result<BrowserResponse, BrowserError> {
        let (binding_events, response_events) = self.wire_page().await?;
        self.collect(binding_events, response_events).await
    }

    /// Install the capture hooks, subscribe to the delivery streams, and navigate.
    async fn wire_page(
        &mut self,
    ) -> Result<
        (
            EventStream<EventBindingCalled>,
            EventStream<EventResponseReceived>,
        ),
        BrowserError,
    > {
        self.install().await?;
        let binding_events = transport(
            self.page.event_listener::<EventBindingCalled>().await,
            "binding_listener",
        )?;
        let response_events = transport(
            self.page.event_listener::<EventResponseReceived>().await,
            "response_listener",
        )?;
        let ui_url = build_ui_url(self.source_origin, self.action)?;
        transport(
            self.page.goto(NavigateParams::new(ui_url)).await,
            "navigate",
        )?;
        Ok((binding_events, response_events))
    }

    /// Enable the domains the capture needs, then install the interceptor script
    /// and the binding that delivers the payload.
    async fn install(&mut self) -> Result<(), BrowserError> {
        self.enable_domains().await?;
        self.install_interceptor().await
    }

    /// Enable Network and Runtime with the capture's own settings.
    ///
    /// Default Network.enable. A durable 32 MiB event buffer makes Chromium
    /// replay oversized buffered messages that the CDP client cannot parse
    /// ("WS Invalid message"), which desynchronises command responses and
    /// surfaces as transport failures. The ranking payload is captured by
    /// the injected binding, so no buffered replay is required.
    async fn enable_domains(&self) -> Result<(), BrowserError> {
        transport(
            self.page
                .execute(chromiumoxide::cdp::browser_protocol::network::EnableParams::default())
                .await,
            "network.enable",
        )?;
        transport(
            self.page
                .execute(chromiumoxide::cdp::js_protocol::runtime::EnableParams::default())
                .await,
            "runtime.enable",
        )?;
        Ok(())
    }

    /// Install the interceptor script, then the binding that delivers the payload.
    ///
    /// `binding_attempted` is raised before the binding call, because cleanup MUST remove the
    /// binding whenever its installation may have landed.
    async fn install_interceptor(&mut self) -> Result<(), BrowserError> {
        let installed = transport(
            self.page
                .execute(
                    AddScriptToEvaluateOnNewDocumentParams::builder()
                        .source(std::mem::take(&mut self.script))
                        .build()
                        .map_err(|_| BrowserError::Protocol)?,
                )
                .await,
            "add_interceptor_script",
        )?;
        self.script_id = Some(installed.identifier.clone());
        self.binding_attempted = true;
        transport(
            self.page
                .execute(
                    AddBindingParams::builder()
                        .name(BINDING_NAME)
                        .build()
                        .map_err(|_| BrowserError::Protocol)?,
                )
                .await,
            "add_binding",
        )?;
        Ok(())
    }

    /// Fold the binding and response streams into ranked candidates until one is
    /// captured, bounded by the attempt deadline.
    async fn collect(
        &self,
        binding_events: EventStream<EventBindingCalled>,
        response_events: EventStream<EventResponseReceived>,
    ) -> Result<BrowserResponse, BrowserError> {
        let events = futures::stream::select(
            binding_events
                .map(CaptureEvent::Binding)
                .chain(futures::stream::once(async { CaptureEvent::Closed })),
            response_events
                .map(CaptureEvent::Response)
                .chain(futures::stream::once(async { CaptureEvent::Closed })),
        )
        .take(MAX_CAPTURE_EVENTS)
        .take_until(tokio::time::sleep_until(self.deadline));
        futures::pin_mut!(events);
        let capture_context = CaptureContext {
            page: self.page,
            action: self.action,
            origin: self.origin,
            nonce: self.nonce,
            deadline: self.deadline,
            gate: self.gate,
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
        build_response(captured)
    }
}
