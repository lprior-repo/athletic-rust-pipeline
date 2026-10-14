mod binding;
mod completion;
mod package;

use self::binding::{process_binding_event, try_resolve_pending};
use self::package::{observe_response_event, route_path_matches, validate_route};
use super::rankings_helper::CapturedRanking;
use crate::protocol::RankingsCapture;
use crate::request::RankingsAction;
use crate::{gate::ProfileGate, BrowserError};
use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    RequestId,
};
use chromiumoxide::cdp::js_protocol::runtime::EventBindingCalled;
use chromiumoxide::Page;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(super) enum CaptureEvent {
    Binding(Arc<EventBindingCalled>),
    Request(Arc<EventRequestWillBeSent>),
    Response(Arc<EventResponseReceived>),
    Finished(Arc<EventLoadingFinished>),
    Failed(Arc<EventLoadingFailed>),
    Closed,
}

pub(super) struct CaptureContext<'a> {
    pub(super) page: &'a Page,
    pub(super) action: &'a RankingsAction,
    pub(super) origin: &'a str,
    pub(super) nonce: u64,
    pub(super) deadline: tokio::time::Instant,
    pub(super) gate: &'a ProfileGate,
}

const CANCELED_LOADING: &str = "net::ERR_ABORTED";

struct WireRequest {
    id: RequestId,
    url: String,
    method: String,
}

struct PendingBinding {
    url: String,
    method: String,
    package: serde_json::Value,
}

pub(super) struct WireLedger {
    requests: Vec<WireRequest>,
    finished: HashMap<RequestId, f64>,
    failed: HashSet<RequestId>,
    redirects: HashSet<RequestId>,
    pending: Vec<PendingBinding>,
}

impl WireLedger {
    pub(super) fn new() -> Self {
        Self {
            requests: Vec::new(),
            finished: HashMap::new(),
            failed: HashSet::new(),
            redirects: HashSet::new(),
            pending: Vec::new(),
        }
    }

    pub(super) fn note_request(
        &mut self,
        id: RequestId,
        url: String,
        method: String,
        has_post_data: Option<bool>,
        has_redirect: bool,
        origin: &str,
        capture: RankingsCapture,
    ) {
        if !validate_route(&url, origin, capture.clone(), &method) {
            return;
        }
        if capture == RankingsCapture::Navigation && has_post_data == Some(true) {
            return;
        }
        if has_redirect {
            self.redirects.insert(id.clone());
        }
        if self.requests.iter().any(|existing| existing.id == id) {
            return;
        }
        self.requests.push(WireRequest { id, url, method });
    }

    pub(super) fn note_response(
        &mut self,
        id: RequestId,
        url: String,
        status: i64,
        origin: &str,
        capture: RankingsCapture,
    ) {
        if !route_path_matches(&url, origin, capture) {
            return;
        }
        if (300..400).contains(&status) {
            self.redirects.insert(id);
        }
    }

    pub(super) fn note_finished(&mut self, id: RequestId, encoded: f64) {
        self.finished.insert(id, encoded);
    }

    pub(super) fn note_failed(
        &mut self,
        id: RequestId,
        error_text: String,
        canceled: Option<bool>,
    ) {
        if canceled == Some(true) || error_text == CANCELED_LOADING {
            self.requests.retain(|existing| existing.id != id);
            self.finished.remove(&id);
            self.failed.remove(&id);
            self.redirects.remove(&id);
            return;
        }
        self.failed.insert(id);
    }

    pub(super) fn candidates_for(&self, url: &str, method: &str) -> Vec<RequestId> {
        self.requests
            .iter()
            .filter(|existing| existing.url == url && existing.method.eq_ignore_ascii_case(method))
            .map(|existing| existing.id.clone())
            .collect()
    }

    pub(super) fn pending_count_for(&self, url: &str, method: &str) -> usize {
        self.pending
            .iter()
            .filter(|existing| existing.url == url && existing.method.eq_ignore_ascii_case(method))
            .count()
    }
}

pub(super) fn uniquely_bound(
    candidates: Vec<RequestId>,
    pending_count: usize,
) -> Result<Option<RequestId>, BrowserError> {
    if pending_count != 1 {
        return Err(BrowserError::Protocol);
    }
    let mut iter = candidates.into_iter();
    let Some(single) = iter.next() else {
        return Ok(None);
    };
    if iter.next().is_some() {
        return Err(BrowserError::Protocol);
    }
    Ok(Some(single))
}

pub(super) async fn process_capture_event(
    event: CaptureEvent,
    context: &CaptureContext<'_>,
    page_one_seen: &mut bool,
    ledger: &mut WireLedger,
) -> Result<Option<CapturedRanking>, BrowserError> {
    match event {
        CaptureEvent::Binding(binding) => {
            process_binding_event(&binding, context, page_one_seen, ledger).await
        }
        CaptureEvent::Request(request) => {
            ledger.note_request(
                request.request_id.clone(),
                request.request.url.clone(),
                request.request.method.clone(),
                request.request.has_post_data,
                request.redirect_response.is_some(),
                context.origin,
                context.action.capture.clone(),
            );
            try_resolve_pending(context, page_one_seen, ledger).await
        }
        CaptureEvent::Response(response) => {
            observe_response_event(
                &response,
                context.action.capture.clone(),
                context.origin,
                context.gate,
            );
            ledger.note_response(
                response.request_id.clone(),
                response.response.url.clone(),
                response.response.status,
                context.origin,
                context.action.capture.clone(),
            );
            try_resolve_pending(context, page_one_seen, ledger).await
        }
        CaptureEvent::Finished(finished) => {
            ledger.note_finished(finished.request_id.clone(), finished.encoded_data_length);
            try_resolve_pending(context, page_one_seen, ledger).await
        }
        CaptureEvent::Failed(failed) => {
            ledger.note_failed(
                failed.request_id.clone(),
                failed.error_text.clone(),
                failed.canceled,
            );
            try_resolve_pending(context, page_one_seen, ledger).await
        }
        CaptureEvent::Closed => Err(BrowserError::Transport),
    }
}
