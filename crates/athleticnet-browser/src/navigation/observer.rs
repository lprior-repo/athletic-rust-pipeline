use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use chromiumoxide::cdp::browser_protocol::network::{
    self, EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    RequestId, ResourceType,
};
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use tokio::time::timeout;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use super::super::{gate::ProfileGate, transport, BrowserError};
use super::REDIRECT_ABORT;
use crate::challenge::{cf_header_challenge, html_body_challenge};
use crate::protocol::MAX_SOURCE_RESPONSE_BYTES;

/// Timeout for body capture futures.
const BODY_CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub(super) struct Observation {
    pub(super) url: String,
    pub(super) status: Option<u16>,
    pub(super) headers: HeaderMap,
    pub(super) challenged: bool,
    pub(super) body_challenged: bool,
    pub(super) body_complete: bool,
    pub(super) failed: bool,
    pub(super) denied: bool,
}

pub(crate) struct PageObserver {
    page: Page,
    stop: CancellationToken,
    gate: Arc<ProfileGate>,
    main_frame: FrameId,
    requests: EventStream<EventRequestWillBeSent>,
    responses: EventStream<EventResponseReceived>,
    finished: EventStream<EventLoadingFinished>,
    failures: EventStream<EventLoadingFailed>,
}

static OBSERVATIONS: LazyLock<Mutex<HashMap<String, Observation>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Start a continuous observer that monitors page navigation events.
///
/// Shutdown uses the stop CancellationToken (not gate.closed()) so that
/// a challenge does not stop the continuous human/auto observer. The
/// observer signals profile challenge by calling gate.revoke() while
/// continuing to observe.
pub(crate) async fn start_observer(
    page: Page,
    stop: CancellationToken,
    gate: Arc<ProfileGate>,
) -> Result<PageObserver, BrowserError> {
    let max_resource_buffer_size = match i64::try_from(MAX_SOURCE_RESPONSE_BYTES) {
        Ok(v) => v,
        Err(_) => return Err(BrowserError::Protocol),
    };
    let max_total_buffer_size = match max_resource_buffer_size.checked_mul(2) {
        Some(v) => v,
        None => return Err(BrowserError::Protocol),
    };
    page.execute(
        network::EnableParams::builder()
            .max_resource_buffer_size(max_resource_buffer_size)
            .max_total_buffer_size(max_total_buffer_size)
            .build(),
    )
    .await
    .map_err(|_| BrowserError::Transport)?;
    let main_frame = page
        .mainframe()
        .await
        .map_err(|_| BrowserError::Transport)?
        .ok_or(BrowserError::Unavailable)?;
    let requests = page
        .event_listener::<EventRequestWillBeSent>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let responses = page
        .event_listener::<EventResponseReceived>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let finished = page
        .event_listener::<EventLoadingFinished>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    let failures = page
        .event_listener::<EventLoadingFailed>()
        .await
        .map_err(|_| BrowserError::Transport)?;
    Ok(PageObserver {
        page,
        stop,
        gate,
        main_frame,
        requests,
        responses,
        finished,
        failures,
    })
}

impl PageObserver {
    pub(crate) async fn run(mut self) -> Result<(), BrowserError> {
        let result = self.run_loop().await;
        let cleanup = remove_observation(&self.page);
        result.and(cleanup)
    }

    async fn run_loop(&mut self) -> Result<(), BrowserError> {
        let cancelled = self.stop.cancelled();
        tokio::pin!(cancelled);
        let mut latest_request_id: Option<RequestId> = None;
        let mut observation = empty_observation(String::new());
        let mut body_futures = FuturesUnordered::new();
        loop {
            tokio::select! {
                biased;
                _ = &mut cancelled => return Ok(()),
                event = self.requests.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if !is_main_document_for(&event, &self.main_frame) { continue; }
                    latest_request_id = Some(event.request_id.clone());
                    observation = empty_observation(event.request.url.clone());
                    body_futures.clear();
                }
                event = self.responses.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if !is_current(&latest_request_id, &event.request_id) { continue; }
                    if latest_request_id.is_some() {
                        self.record_status(&mut observation, &event)?;
                    }
                }
                event = self.finished.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if !is_current(&latest_request_id, &event.request_id) { continue; }
                    if latest_request_id.is_some() {
                        let id = event.request_id.clone();
                        body_futures.push(capture_body_bounded(self.page.clone(), id));
                    }
                }
                event = self.failures.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if !is_current(&latest_request_id, &event.request_id) { continue; }
                    if latest_request_id.is_some() {
                        if event.error_text == REDIRECT_ABORT || event.canceled == Some(true) {
                            continue;
                        }
                        observation.failed = true;
                        observation.body_complete = true;
                        store_observation(&self.page, observation.clone())?;
                    }
                }
                result = body_futures.next(), if !body_futures.is_empty() => {
                    let result: Option<(RequestId, Result<Vec<u8>, BrowserError>)> = result;
                    let Some((request_id, body_result)) = result else { continue };
                    if latest_request_id.as_ref() != Some(&request_id) {
                        continue;
                    }
                    match body_result {
                        Ok(body) => self.record_body(&mut observation, &body)?,
                        Err(e) => tracing::debug!("body capture failed: {e}"),
                    }
                }
            }
        }
    }

    /// Fold response headers into the tracked observation, revoking on denial
    /// or header challenge.
    fn record_status(
        &self,
        observation: &mut Observation,
        event: &EventResponseReceived,
    ) -> Result<(), BrowserError> {
        observation.status = Some(navigation_status(event.response.status)?);
        observation.headers = transport::response_headers(event)?;
        if let Some(status) = observation.status {
            if status == 403 || status == 429 {
                observation.denied = true;
                self.gate.revoke();
            }
        }
        observation.challenged = cf_header_challenge(&observation.headers);
        if observation.challenged {
            self.gate.revoke();
        }
        Ok(())
    }

    /// Fold a captured body into the tracked observation, revoking on challenge.
    fn record_body(&self, observation: &mut Observation, body: &[u8]) -> Result<(), BrowserError> {
        observation.body_challenged = html_body_challenge(
            observation
                .headers
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map_or("", |value| value),
            body,
        );
        if observation.body_challenged {
            self.gate.revoke();
        }
        observation.body_complete = true;
        store_observation(&self.page, observation.clone())
    }
}

/// Capture one response body, bounded by the body-capture timeout.
async fn capture_body_bounded(
    page: Page,
    id: RequestId,
) -> (RequestId, Result<Vec<u8>, BrowserError>) {
    let body_result = timeout(
        BODY_CAPTURE_TIMEOUT,
        transport::capture_body(&page, id.clone()),
    )
    .await
    .map_err(|_| BrowserError::Timeout)
    .and_then(|body| body);
    (id, body_result)
}

/// True when the event belongs to the document the observer is tracking.
pub(super) fn is_current(latest: &Option<RequestId>, event: &RequestId) -> bool {
    latest.as_ref().is_none_or(|current| current == event)
}

pub(super) fn is_main_document_for(event: &EventRequestWillBeSent, main_frame: &FrameId) -> bool {
    event.r#type.as_ref() == Some(&ResourceType::Document)
        && event.frame_id.as_ref() == Some(main_frame)
}

pub(super) fn navigation_status(status: i64) -> Result<u16, BrowserError> {
    u16::try_from(status).map_err(|_| BrowserError::Protocol)
}

pub(super) fn empty_observation(url: String) -> Observation {
    Observation {
        url,
        status: None,
        headers: HeaderMap::new(),
        challenged: false,
        body_challenged: false,
        body_complete: false,
        failed: false,
        denied: false,
    }
}

fn observation_key(page: &Page) -> String {
    page.target_id().inner().to_string()
}

pub(super) fn store_observation(page: &Page, observation: Observation) -> Result<(), BrowserError> {
    let mut values = OBSERVATIONS.lock().map_err(|_| BrowserError::Protocol)?;
    let key = observation_key(page);
    if !values.contains_key(&key) && values.len() >= 8 {
        let old_key = values
            .keys()
            .next()
            .cloned()
            .ok_or(BrowserError::Protocol)?;
        values.remove(&old_key);
    }
    values.insert(key, observation);
    Ok(())
}

pub(super) fn load_observation(page: &Page) -> Result<Option<Observation>, BrowserError> {
    let values = OBSERVATIONS.lock().map_err(|_| BrowserError::Protocol)?;
    Ok(values.get(&observation_key(page)).cloned())
}

fn remove_observation(page: &Page) -> Result<(), BrowserError> {
    let mut values = OBSERVATIONS.lock().map_err(|_| BrowserError::Protocol)?;
    values.remove(&observation_key(page));
    Ok(())
}
