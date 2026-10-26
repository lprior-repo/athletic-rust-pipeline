use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
    EventResponseReceived, EventResponseReceivedExtraInfo, RequestId,
};
use chromiumoxide::Page;
use reqwest::header::CONTENT_TYPE;
use tokio::time::timeout;

use super::same_site::{hop_within_origin, MAX_REDIRECT_HOPS};
use super::transfer::refuse_excess_wire;
use super::{
    capture, matches_request, record_response, request_method, FetchResult, ResponseEvidence,
};
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
    same_site_origin: Option<url::Url>,
    expected_url: String,
    redirect_hops: u8,
    awaiting_hop: bool,
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
        same_site_origin: Option<url::Url>,
    ) -> Self {
        Self {
            page,
            gate,
            request,
            request_body,
            invocation,
            same_site_origin,
            expected_url: request.url.as_str().to_string(),
            redirect_hops: 0,
            awaiting_hop: false,
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
        if event
            .redirect_response
            .as_ref()
            .is_some_and(|redirected| redirected.url == self.expected_url)
        {
            return self.follow_hop(&event, remaining).await;
        }
        if !matches_request(&event, self.request, &self.expected_url)
            || !self.invocation.binding().matches(&event)
        {
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
        self.bind_tracked(&event).await
    }

    /// Re-anchor tracking onto a same-site redirect hop.
    ///
    /// The hop's linkage is the `redirect_response` naming the URL the lane
    /// already tracks: CDP events come from the browser, never the page, so
    /// a page cannot forge this handoff. The isolated-world binding check is
    /// skipped for the hop itself because Chromium reissues redirect hops
    /// outside the script frame; the body confirm below still runs.
    async fn follow_hop(
        &mut self,
        event: &Arc<EventRequestWillBeSent>,
        remaining: Duration,
    ) -> Result<(), BrowserError> {
        let Some(origin) = self.same_site_origin.clone() else {
            tracing::debug!("lane refusing redirect: same-site following disabled");
            return Err(BrowserError::Redirect);
        };
        if self.redirect_hops >= MAX_REDIRECT_HOPS {
            tracing::debug!(
                hops = self.redirect_hops,
                "lane refusing redirect: hop budget spent"
            );
            return Err(BrowserError::Redirect);
        }
        if !hop_within_origin(&event.request.url, &origin) {
            tracing::debug!(
                hop = event.request.url.as_str(),
                "lane refusing redirect: hop leaves the bound site"
            );
            return Err(BrowserError::Redirect);
        }
        if !event
            .request
            .method
            .eq_ignore_ascii_case(request_method(self.request))
        {
            tracing::debug!("lane refusing redirect: method changed across the hop");
            return Err(BrowserError::Redirect);
        }
        tracing::debug!(
            hop = event.request.url.as_str(),
            hops = self.redirect_hops,
            "lane following same-site redirect"
        );
        let confirm = timeout(
            remaining,
            capture::matches_request_body(self.page, event, self.request_body),
        )
        .await;
        match confirm {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => return Ok(()),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(BrowserError::Timeout),
        }
        self.redirect_hops = self.redirect_hops.saturating_add(1);
        self.expected_url = event.request.url.clone();
        self.request_id = None;
        self.response = None;
        self.pending = PendingTransfers::default();
        self.progress = WireTransfers::default();
        self.encoded_total = None;
        self.loading_finished = false;
        self.redirect_status = None;
        self.awaiting_hop = false;
        self.bind_tracked(event).await
    }

    async fn bind_tracked(
        &mut self,
        event: &Arc<EventRequestWillBeSent>,
    ) -> Result<(), BrowserError> {
        let id = event.request_id.clone();
        self.request_id = Some(id.clone());
        self.progress.bind(&id);
        if let Some(pending) = std::mem::take(&mut self.pending).take(&id) {
            if let Some(response) = pending.response {
                self.verify_response_url(&response)?;
                if self.defer_redirect(&response) {
                    self.awaiting_hop = true;
                } else {
                    record_response(
                        &response,
                        &mut self.response,
                        &mut self.challenge_seen,
                        self.gate,
                    )?;
                }
            }
            if let Some(encoded) = pending.finished {
                if !self.awaiting_hop {
                    self.progress.finish(&id, encoded)?;
                    self.encoded_total = Some(encoded);
                    self.loading_finished = true;
                }
            }
        }
        Ok(())
    }

    /// Hold a same-site 3xx for its redirect hop instead of recording it.
    ///
    /// The 3xx response can arrive before the tracked request binds (the
    /// bind confirm round-trips through the page), so both the live and the
    /// pending-drain paths defer through here. Anything the hop rule rejects
    /// still fails as a redirect exactly as before.
    fn defer_redirect(&self, event: &EventResponseReceived) -> bool {
        let status: u16 = match event.response.status.try_into() {
            Ok(status) => status,
            Err(_) => return false,
        };
        let defer = self.same_site_origin.is_some()
            && (300..400).contains(&status)
            && event.response.url == self.expected_url;
        if defer {
            tracing::debug!("lane deferring redirect response, awaiting hop");
        }
        defer
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
            if self.defer_redirect(&event) {
                self.awaiting_hop = true;
                return Ok(());
            }
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
            if self.awaiting_hop {
                return Err(BrowserError::Redirect);
            }
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
        if event.response.url != self.expected_url {
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
