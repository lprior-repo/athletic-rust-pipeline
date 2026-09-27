use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    RequestId,
};
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use tokio::time::Instant;

use super::observer::Observation;
use super::observer::{empty_observation, is_current, is_main_document_for, navigation_status};
use super::REDIRECT_ABORT;
use crate::challenge::{cf_header_challenge, html_body_challenge};
use crate::{gate::ProfileGate, transport, BrowserError};

pub(super) struct BootstrapEvents {
    pub(super) requests: EventStream<EventRequestWillBeSent>,
    pub(super) responses: EventStream<EventResponseReceived>,
    pub(super) finished: EventStream<EventLoadingFinished>,
    pub(super) failures: EventStream<EventLoadingFailed>,
}

pub(super) async fn bootstrap_events(page: &Page) -> Result<BootstrapEvents, BrowserError> {
    Ok(BootstrapEvents {
        requests: transport::subscribe(page).await?,
        responses: transport::subscribe(page).await?,
        finished: transport::subscribe(page).await?,
        failures: transport::subscribe(page).await?,
    })
}

pub(super) fn navigation_deadline(now: Instant, timeout: Duration) -> Instant {
    now.checked_add(timeout)
        .unwrap_or_else(|| now.checked_add(Duration::from_secs(300)).unwrap_or(now))
}

pub(super) struct DocumentState {
    navigation_done: bool,
    latest_request_id: Option<RequestId>,
    document_id: Option<RequestId>,
    observation: Observation,
    pending_response: Option<Arc<EventResponseReceived>>,
    pending_finished: Option<RequestId>,
}

impl DocumentState {
    pub(super) fn new(url: String) -> Self {
        Self {
            navigation_done: false,
            latest_request_id: None,
            document_id: None,
            observation: empty_observation(url),
            pending_response: None,
            pending_finished: None,
        }
    }

    pub(super) fn complete(&self) -> bool {
        self.navigation_done && self.observation.status.is_some() && self.observation.body_complete
    }

    pub(super) fn navigated(&self) -> bool {
        self.navigation_done
    }

    pub(super) fn mark_navigated(&mut self) {
        self.navigation_done = true;
    }

    pub(super) fn into_observation(self) -> Observation {
        self.observation
    }

    pub(super) async fn on_request(
        &mut self,
        page: &Page,
        event: &EventRequestWillBeSent,
        main_frame: &FrameId,
    ) -> Result<(), BrowserError> {
        if !is_main_document_for(event, main_frame) {
            return Ok(());
        }
        let id = event.request_id.clone();
        self.latest_request_id = Some(id.clone());
        self.document_id = Some(id.clone());
        self.observation.url = event.request.url.clone();
        self.observation.status = None;
        self.observation.headers = HeaderMap::new();
        self.observation.challenged = false;
        self.observation.body_challenged = false;
        self.observation.body_complete = false;
        self.observation.failed = false;
        self.pending_response.take();
        if self.pending_finished.as_ref() == Some(&id) {
            let body = transport::capture_body(page, id.clone()).await?;
            record_body(&mut self.observation, &body);
        }
        Ok(())
    }

    pub(super) fn on_response(
        &mut self,
        event: Arc<EventResponseReceived>,
        gate: &ProfileGate,
    ) -> Result<(), BrowserError> {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return Ok(());
        }
        if self.document_id.as_ref() == Some(&event.request_id) {
            record_headers(&mut self.observation, &event, gate)?;
        } else if self.document_id.is_none() {
            self.pending_response = Some(event);
        }
        Ok(())
    }

    pub(super) async fn on_finished(
        &mut self,
        page: &Page,
        event: Arc<EventLoadingFinished>,
    ) -> Result<(), BrowserError> {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return Ok(());
        }
        if self.document_id.as_ref() == Some(&event.request_id) {
            let body = transport::capture_body(page, event.request_id.clone()).await?;
            record_body(&mut self.observation, &body);
        } else if self.document_id.is_none() {
            self.pending_finished = Some(event.request_id.clone());
        }
        Ok(())
    }

    pub(super) fn on_failure(&mut self, event: &EventLoadingFailed) {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return;
        }
        if self.document_id.as_ref() != Some(&event.request_id) {
            return;
        }
        if event.error_text == REDIRECT_ABORT || event.canceled == Some(true) {
            return;
        }
        self.observation.failed = true;
        self.observation.body_complete = true;
    }
}

fn record_body(observation: &mut Observation, body: &[u8]) {
    let media_type = observation
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map_or("", |value| value);
    observation.body_challenged = html_body_challenge(media_type, body);
    observation.body_complete = true;
}

fn record_headers(
    observation: &mut Observation,
    event: &EventResponseReceived,
    gate: &ProfileGate,
) -> Result<(), BrowserError> {
    observation.status = Some(navigation_status(event.response.status)?);
    observation.headers = transport::response_headers(event)?;
    observation.challenged = cf_header_challenge(&observation.headers);
    if observation.challenged {
        gate.revoke();
    }
    Ok(())
}
