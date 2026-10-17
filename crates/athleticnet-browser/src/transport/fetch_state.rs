use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
    EventResponseReceived, EventResponseReceivedExtraInfo, RequestId,
};
use chromiumoxide::Page;
use reqwest::header::CONTENT_TYPE;
use tokio::time::timeout;

use super::transfer::refuse_excess_wire;
use super::{capture, matches_request, record_response, FetchResult, ResponseEvidence};
use crate::challenge::html_body_challenge;
use crate::request::RequestSpec;
use crate::{gate::ProfileGate, BrowserError, BrowserResponse};

use super::invocation::Invocation;
use super::pending::PendingTransfers;
use super::progress::WireTransfers;

pub(super) enum FetchEvent {
    Evaluation(Result<FetchResult, BrowserError>),
    Request(Arc<EventRequestWillBeSent>),
    Response(Arc<EventResponseReceived>),
    ExtraInfo(Arc<EventResponseReceivedExtraInfo>),
    Finished(Arc<EventLoadingFinished>),
    Data(Arc<EventDataReceived>),
    Failed(Arc<EventLoadingFailed>),
    Closed,
    Timer,
}

pub(super) struct FetchCapture<'a> {
    pub(super) page: &'a Page,
    pub(super) gate: &'a ProfileGate,
    pub(super) request: &'a RequestSpec,
    pub(super) request_body: Option<&'a str>,
    pub(super) invocation: &'a Invocation,
    request_id: Option<RequestId>,
    pending: PendingTransfers,
    encoded_total: Option<f64>,
    progress: WireTransfers,
    response: Option<ResponseEvidence>,
    loading_finished: bool,
    evaluation_result: Option<FetchResult>,
    challenge_seen: bool,
    redirect_status: Option<u16>,
}

impl<'a> FetchCapture<'a> {
    pub(super) fn new(
        page: &'a Page,
        gate: &'a ProfileGate,
        request: &'a RequestSpec,
        request_body: Option<&'a str>,
        invocation: &'a Invocation,
    ) -> Self {
        Self {
            page,
            gate,
            request,
            request_body,
            invocation,
            request_id: None,
            pending: PendingTransfers::default(),
            encoded_total: None,
            progress: WireTransfers::default(),
            response: None,
            loading_finished: false,
            evaluation_result: None,
            challenge_seen: false,
            redirect_status: None,
        }
    }

    pub(super) fn awaiting_evaluation(&self) -> bool {
        self.evaluation_result.is_none()
    }

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
            FetchEvent::Finished(event) => self.on_finished(event)?,
            FetchEvent::Data(event) => self.on_data(&event)?,
            FetchEvent::Failed(event) => self.on_failed(&event)?,
            FetchEvent::Closed => return Err(BrowserError::Transport),
            FetchEvent::Timer => return Err(BrowserError::Timeout),
        }
        Ok(())
    }

    pub(super) fn complete(&self) -> Result<bool, BrowserError> {
        if let Some(result) = self.evaluation_result.as_ref() {
            result.validate()?;
        }
        if self.evaluation_result.is_some() && self.response.is_some() && self.loading_finished {
            return Ok(true);
        }
        Ok(false)
    }

    async fn on_request(
        &mut self,
        event: Arc<EventRequestWillBeSent>,
        remaining: Duration,
    ) -> Result<(), BrowserError> {
        if self.request_id.as_ref() == Some(&event.request_id) {
            return Err(if event.redirect_response.is_some() {
                BrowserError::Redirect
            } else {
                BrowserError::Protocol
            });
        }
        if !matches_request(&event, self.request) || !self.invocation.binding().matches(&event) {
            return Ok(());
        }
        if self.request_id.is_some() {
            return Err(BrowserError::Protocol);
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
        let id = event.request_id.clone();
        self.request_id = Some(id.clone());
        self.progress.bind(&id);
        if let Some(pending) = std::mem::take(&mut self.pending).take(&id) {
            if let Some(response) = pending.response {
                self.verify_response_url(&response)?;
                record_response(
                    &response,
                    &mut self.response,
                    &mut self.challenge_seen,
                    self.gate,
                )?;
            }
            if let Some(encoded) = pending.finished {
                self.progress.finish(&id, encoded)?;
                self.encoded_total = Some(encoded);
                self.loading_finished = true;
            }
        }
        Ok(())
    }

    fn on_response(&mut self, event: Arc<EventResponseReceived>) -> Result<(), BrowserError> {
        if self
            .request_id
            .as_ref()
            .is_some_and(|known| known != &event.request_id)
        {
            return Ok(());
        }
        if self.request_id.as_ref() == Some(&event.request_id) {
            if self.response.is_some() {
                return Err(BrowserError::Protocol);
            }
            self.verify_response_url(&event)?;
            record_response(
                &event,
                &mut self.response,
                &mut self.challenge_seen,
                self.gate,
            )?;
        } else if self.request_id.is_none() {
            self.pending.observe_response(event)?;
        }
        Ok(())
    }

    fn on_extra_info(&mut self, event: &EventResponseReceivedExtraInfo) {
        if let Some(current_id) = &self.request_id {
            if current_id == &event.request_id && (300..400).contains(&event.status_code) {
                self.redirect_status = u16::try_from(event.status_code).ok();
            }
        }
    }

    fn on_data(&mut self, event: &EventDataReceived) -> Result<(), BrowserError> {
        if self
            .request_id
            .as_ref()
            .is_some_and(|id| id != &event.request_id)
        {
            return Ok(());
        }
        self.progress.observe(
            &event.request_id,
            event.encoded_data_length,
            event.data_length,
        )
    }

    fn on_finished(&mut self, event: Arc<EventLoadingFinished>) -> Result<(), BrowserError> {
        if self
            .request_id
            .as_ref()
            .is_some_and(|known| known != &event.request_id)
        {
            return Ok(());
        }
        if self.request_id.as_ref() == Some(&event.request_id) {
            self.progress
                .finish(&event.request_id, event.encoded_data_length)?;
            self.loading_finished = true;
            self.encoded_total = Some(event.encoded_data_length);
        } else if self.request_id.is_none() {
            self.pending
                .observe_finished(&event.request_id, event.encoded_data_length)?;
        }
        Ok(())
    }

    fn on_failed(&mut self, event: &EventLoadingFailed) -> Result<(), BrowserError> {
        if self.request_id.as_ref() == Some(&event.request_id) {
            if self.redirect_status.take().is_some() {
                return Err(BrowserError::Redirect);
            }
            return Err(BrowserError::Transport);
        }
        Ok(())
    }

    fn verify_response_url(&self, event: &EventResponseReceived) -> Result<(), BrowserError> {
        if event.response.url != self.request.url.as_str() {
            return Err(BrowserError::Redirect);
        }
        Ok(())
    }
}

pub(super) async fn complete_fetch(
    capture: FetchCapture<'_>,
) -> Result<BrowserResponse, BrowserError> {
    let page = capture.page;
    let gate = capture.gate;
    if !capture.loading_finished {
        return Err(BrowserError::Timeout);
    }
    let evidence = capture.response.ok_or(BrowserError::Transport)?;
    let result = capture.evaluation_result.ok_or(BrowserError::Transport)?;
    result.validate()?;
    if result.status != Some(evidence.status) {
        return Err(BrowserError::Protocol);
    }
    refuse_excess_wire(capture.encoded_total)?;
    let request_id = capture.request_id.ok_or(BrowserError::Protocol)?;
    let body = capture::capture_body(page, request_id).await?;
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
        response_url: Some(evidence.response_url),
        headers: evidence.headers,
        body,
        rankings: None,
    })
}
