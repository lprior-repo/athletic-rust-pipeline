use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
    EventResponseReceived, RequestId,
};
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use reqwest::header::CONTENT_TYPE;
use tokio::time::Instant;

use super::observer::{
    empty_observation, is_current, is_main_document_for, navigation_status, Observation,
};
use super::REDIRECT_ABORT;
use crate::challenge::{cf_header_challenge, html_body_challenge};
use crate::gate::ProfileGate;
use crate::transport;
use crate::transport::pending::PendingTransfers;
use crate::transport::progress::WireTransfers;
use crate::BrowserError;

pub(super) struct BootstrapEvents {
    pub(super) requests: EventStream<EventRequestWillBeSent>,
    pub(super) responses: EventStream<EventResponseReceived>,
    pub(super) finished: EventStream<EventLoadingFinished>,
    pub(super) failures: EventStream<EventLoadingFailed>,
    pub(super) data: EventStream<EventDataReceived>,
}

pub(super) async fn bootstrap_events(page: &Page) -> Result<BootstrapEvents, BrowserError> {
    Ok(BootstrapEvents {
        requests: transport::subscribe(page).await?,
        responses: transport::subscribe(page).await?,
        finished: transport::subscribe(page).await?,
        failures: transport::subscribe(page).await?,
        data: transport::subscribe(page).await?,
    })
}

pub(super) fn navigation_deadline(now: Instant, timeout: Duration) -> Instant {
    match now.checked_add(timeout) {
        Some(value) => value,
        None => now
            .checked_add(Duration::from_secs(300))
            .map_or(now, |value| value),
    }
}

pub(super) struct DocumentState {
    navigation_done: bool,
    latest_request_id: Option<RequestId>,
    observation: Observation,
    pending: PendingTransfers,
    progress: WireTransfers,
}

impl DocumentState {
    pub(super) fn new(url: String) -> Self {
        Self {
            navigation_done: false,
            latest_request_id: None,
            observation: empty_observation(url),
            pending: PendingTransfers::default(),
            progress: WireTransfers::default(),
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
        let capture_id = self.accept_main_request(event, main_frame)?;
        if let Some(id) = capture_id {
            let body = transport::capture_body(page, id).await?;
            record_body(&mut self.observation, &body);
        }
        Ok(())
    }

    pub(super) fn accept_main_request(
        &mut self,
        event: &EventRequestWillBeSent,
        main_frame: &FrameId,
    ) -> Result<Option<RequestId>, BrowserError> {
        if !is_main_document_for(event, main_frame) {
            return Ok(None);
        }
        let id = &event.request_id;
        self.latest_request_id = Some(id.clone());
        self.observation = empty_observation(event.request.url.clone());
        self.progress.bind(id);
        if let Some(pending_transfer) = std::mem::take(&mut self.pending).take(id) {
            if let Some(pending_response) = pending_transfer.response {
                self.observation.status =
                    Some(navigation_status(pending_response.response.status)?);
                self.observation.headers = transport::response_headers(&pending_response)?;
                self.observation.challenged = cf_header_challenge(&self.observation.headers);
            }
            if let Some(total) = pending_transfer.finished {
                self.progress.finish(id, total)?;
                return Ok(Some(id.clone()));
            }
        }
        Ok(None)
    }

    pub(super) fn on_response(
        &mut self,
        event: Arc<EventResponseReceived>,
        gate: &ProfileGate,
    ) -> Result<(), BrowserError> {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return Ok(());
        }
        if self.latest_request_id.is_some() {
            record_headers(&mut self.observation, &event, gate)?;
        } else {
            self.pending.observe_response(event)?;
        }
        Ok(())
    }

    pub(super) fn on_data(&mut self, event: &EventDataReceived) -> Result<(), BrowserError> {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return Ok(());
        }
        self.progress.observe(
            &event.request_id,
            event.encoded_data_length,
            event.data_length,
        )
    }

    pub(super) async fn on_finished(
        &mut self,
        page: &Page,
        event: Arc<EventLoadingFinished>,
    ) -> Result<(), BrowserError> {
        if let Some(id) = self.accept_finished(&event)? {
            let body = transport::capture_body(page, id).await?;
            record_body(&mut self.observation, &body);
        }
        Ok(())
    }

    fn accept_finished(
        &mut self,
        event: &EventLoadingFinished,
    ) -> Result<Option<RequestId>, BrowserError> {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return Ok(None);
        }
        if self.latest_request_id.is_some() {
            self.progress
                .finish(&event.request_id, event.encoded_data_length)?;
            return Ok(Some(event.request_id.clone()));
        }
        self.pending
            .observe_finished(&event.request_id, event.encoded_data_length)?;
        Ok(None)
    }

    pub(super) fn on_failure(&mut self, event: &EventLoadingFailed) {
        if !is_current(&self.latest_request_id, &event.request_id) {
            return;
        }
        if self.latest_request_id.is_none() {
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

#[cfg(test)]
#[path = "document/tests.rs"]
mod tests;
