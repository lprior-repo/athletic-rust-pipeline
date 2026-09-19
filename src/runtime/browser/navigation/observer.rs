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
use crate::runtime::protocol::MAX_SOURCE_RESPONSE_BYTES;
use crate::runtime::source::http::challenge::{cf_header_challenge, html_body_challenge};

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
    let max_resource_buffer_size = MAX_SOURCE_RESPONSE_BYTES as i64;
    let max_total_buffer_size = (MAX_SOURCE_RESPONSE_BYTES * 2) as i64;
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
        let mut observation = Observation {
            url: String::new(),
            status: None,
            headers: HeaderMap::new(),
            challenged: false,
            body_challenged: false,
            body_complete: false,
            failed: false,
            denied: false,
        };
        let mut body_futures = FuturesUnordered::new();
        loop {
            tokio::select! {
                biased;
                _ = &mut cancelled => return Ok(()),
                event = self.requests.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if !is_main_document_for(&event, &self.main_frame) {
                        continue;
                    }
                    let id = event.request_id.clone();
                    latest_request_id = Some(id.clone());
                    observation = empty_observation(event.request.url.clone());
                    body_futures.clear();
                }
                event = self.responses.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if let Some(latest) = &latest_request_id {
                        if latest != &event.request_id {
                            continue;
                        }
                    }
                    if latest_request_id.as_ref() == Some(&event.request_id) {
                        observation.status = Some(navigation_status(event.response.status)?);
                        observation.headers = transport::response_headers(&event)?;
                        // Observe API denials during browser navigation.
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
                    }
                }
                event = self.finished.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if let Some(latest) = &latest_request_id {
                        if latest != &event.request_id {
                            continue;
                        }
                    }
                    if latest_request_id.as_ref() == Some(&event.request_id) {
                        let page = self.page.clone();
                        let id = event.request_id.clone();
                        let fut = async move {
                            let body_result = timeout(
                                BODY_CAPTURE_TIMEOUT,
                                transport::capture_body(&page, id.clone()),
                            )
                            .await
                            .map_err(|_| BrowserError::Timeout)
                            .and_then(|b| b);
                            (id, body_result)
                        };
                        body_futures.push(fut);
                    }
                }
                event = self.failures.next() => {
                    let Some(event) = event else { return Err(BrowserError::Transport); };
                    if let Some(latest) = &latest_request_id {
                        if latest != &event.request_id {
                            continue;
                        }
                    }
                    if latest_request_id.as_ref() == Some(&event.request_id) {
                        observation.failed = true;
                        observation.body_complete = true;
                        store_observation(&self.page, observation.clone())?;
                    }
                }
                result = body_futures.next(), if !body_futures.is_empty() => {
                    let result: Option<(RequestId, Result<Vec<u8>, BrowserError>)> = result;
                    let Some((request_id, body_result)) = result else { continue };
                    // Verify current RequestId — drop stale results.
                    if latest_request_id.as_ref() != Some(&request_id) {
                        continue;
                    }
                    match body_result {
                        Ok(body) => {
                            observation.body_challenged =
                                html_body_challenge(
                                    observation
                                        .headers
                                        .get(CONTENT_TYPE)
                                        .and_then(|v| v.to_str().ok())
                                        .map_or("", |v| v),
                                    &body,
                                );
                            if observation.body_challenged {
                                self.gate.revoke();
                            }
                            observation.body_complete = true;
                            store_observation(&self.page, observation.clone())?;
                        }
                        Err(e) => {
                            tracing::debug!("body capture failed: {e}");
                        }
                    }
                }
            }
        }
    }
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
