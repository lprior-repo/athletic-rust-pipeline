//! State machine folding CDP events for one in-page fetch.

use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    EventResponseReceivedExtraInfo, RequestId,
};
use chromiumoxide::Page;
use reqwest::header::CONTENT_TYPE;
use tokio::time::timeout;

use super::{capture, fail, matches_request, record_response, FetchResult, ResponseEvidence};
use crate::challenge::html_body_challenge;
use crate::request::RequestSpec;
use crate::{gate::ProfileGate, BrowserError, BrowserResponse};

/// One decoded capture event from the fetch's CDP streams.
pub(super) enum FetchEvent {
    Evaluation(Result<FetchResult, BrowserError>),
    Request(Arc<EventRequestWillBeSent>),
    Response(Arc<EventResponseReceived>),
    ExtraInfo(Arc<EventResponseReceivedExtraInfo>),
    Finished(Arc<EventLoadingFinished>),
    Failed(Arc<EventLoadingFailed>),
    Closed,
    Timer,
}

/// Capture state for one browser fetch.
pub(super) struct FetchCapture<'a> {
    pub(super) page: &'a Page,
    pub(super) gate: &'a ProfileGate,
    pub(super) request: &'a RequestSpec,
    pub(super) request_body: Option<&'a str>,
    request_id: Option<RequestId>,
    pending_response: Option<Arc<EventResponseReceived>>,
    pending_finished: Option<RequestId>,
    response: Option<ResponseEvidence>,
    loading_finished: bool,
    evaluation_result: Option<FetchResult>,
    challenge_seen: bool,
    redirect_status: Option<u16>,
}

impl<'a> FetchCapture<'a> {
    /// Fresh capture state for one request.
    pub(super) fn new(
        page: &'a Page,
        gate: &'a ProfileGate,
        request: &'a RequestSpec,
        request_body: Option<&'a str>,
    ) -> Self {
        Self {
            page,
            gate,
            request,
            request_body,
            request_id: None,
            pending_response: None,
            pending_finished: None,
            response: None,
            loading_finished: false,
            evaluation_result: None,
            challenge_seen: false,
            redirect_status: None,
        }
    }

    /// True while the in-page fetch has not reported its result yet.
    pub(super) fn awaiting_evaluation(&self) -> bool {
        self.evaluation_result.is_none()
    }

    /// Fold one capture event into the state.
    pub(super) async fn apply(
        &mut self,
        event: FetchEvent,
        remaining: Duration,
    ) -> Result<(), BrowserError> {
        match event {
            FetchEvent::Evaluation(result) => self.evaluation_result = Some(result?),
            FetchEvent::Request(event) => self.on_request(event, remaining).await?,
            FetchEvent::Response(event) => self.on_response(event)?,
            FetchEvent::ExtraInfo(event) => self.on_extra_info(&event),
            FetchEvent::Finished(event) => self.on_finished(event),
            FetchEvent::Failed(event) => self.on_failed(&event)?,
            FetchEvent::Closed => return Err(BrowserError::Transport),
            FetchEvent::Timer => return Err(BrowserError::Timeout),
        }
        Ok(())
    }

    /// True once the loop has nothing left to wait for.
    ///
    /// The payload limit is fatal on its own: the JS helper reported it, so no
    /// response can arrive.
    pub(super) fn complete(&self) -> Result<bool, BrowserError> {
        if let Some(result) = self.evaluation_result.as_ref() {
            if !result.ok && result.error.as_deref() == Some("payload_limit") {
                return Err(BrowserError::PayloadLimit);
            }
        }
        if self.evaluation_result.is_some() && self.response.is_some() && self.loading_finished {
            return Ok(true);
        }
        Ok(self.evaluation_result.as_ref().is_some_and(|r| !r.ok) && self.loading_finished)
    }

    /// Confirm the request and adopt a response that arrived before it.
    async fn on_request(
        &mut self,
        event: Arc<EventRequestWillBeSent>,
        remaining: Duration,
    ) -> Result<(), BrowserError> {
        if !matches_request(&event, self.request) {
            return Ok(());
        }
        let id = event.request_id.clone();
        if self.request_id.as_ref().is_some_and(|known| known != &id) {
            return Ok(());
        }
        let confirm = timeout(
            remaining,
            capture::matches_request_body(self.page, &event, self.request_body),
        )
        .await;
        match confirm {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => return Ok(()),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(BrowserError::Timeout),
        }
        if event.redirect_response.is_some() {
            return Err(BrowserError::Redirect);
        }
        if self.request_id.as_ref().is_some_and(|known| known != &id) {
            return Err(BrowserError::Protocol);
        }
        self.request_id = Some(id.clone());
        if let Some(pending) = self.pending_response.take() {
            if pending.request_id == id {
                record_response(
                    &pending,
                    &mut self.response,
                    &mut self.challenge_seen,
                    self.gate,
                )?;
            }
        }
        if self.pending_finished.as_ref() == Some(&id) {
            self.loading_finished = true;
        }
        Ok(())
    }

    /// Adopt the response of the confirmed request, or hold it until then.
    fn on_response(&mut self, event: Arc<EventResponseReceived>) -> Result<(), BrowserError> {
        if self
            .request_id
            .as_ref()
            .is_some_and(|known| known != &event.request_id)
        {
            return Ok(());
        }
        if self.request_id.as_ref() == Some(&event.request_id) {
            record_response(
                &event,
                &mut self.response,
                &mut self.challenge_seen,
                self.gate,
            )?;
        } else if self.request_id.is_none() {
            self.pending_response = Some(event);
        }
        Ok(())
    }

    /// Record the redirect status of the confirmed request.
    ///
    /// Correlated with the confirmed request — redirect status only matters for it.
    fn on_extra_info(&mut self, event: &EventResponseReceivedExtraInfo) {
        if let Some(current_id) = &self.request_id {
            if current_id == &event.request_id && (300..400).contains(&event.status_code) {
                self.redirect_status = u16::try_from(event.status_code).ok();
            }
        }
    }

    /// Note the loading-finished event of the confirmed request.
    fn on_finished(&mut self, event: Arc<EventLoadingFinished>) {
        if self
            .request_id
            .as_ref()
            .is_some_and(|known| known != &event.request_id)
        {
            return;
        }
        if self.request_id.as_ref() == Some(&event.request_id) {
            self.loading_finished = true;
        } else if self.request_id.is_none() {
            self.pending_finished = Some(event.request_id.clone());
        }
    }

    /// Reject the fetch when the confirmed request failed.
    fn on_failed(&mut self, event: &EventLoadingFailed) -> Result<(), BrowserError> {
        if self.request_id.as_ref() == Some(&event.request_id) {
            if self.redirect_status.take().is_some() {
                return Err(BrowserError::Redirect);
            }
            return Err(BrowserError::Transport);
        }
        Ok(())
    }
}

/// Require complete loading, then validate the capture and build the response.
pub(super) async fn complete_fetch(
    capture: FetchCapture<'_>,
) -> Result<BrowserResponse, BrowserError> {
    let page = capture.page;
    let gate = capture.gate;
    if !capture.loading_finished {
        return Err(fail(page, gate, BrowserError::Timeout).await);
    }
    let evidence = capture.response.ok_or(BrowserError::Transport)?;
    let result = capture.evaluation_result.ok_or(BrowserError::Transport)?;
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
    if capture.challenge_seen || body_challenge {
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
