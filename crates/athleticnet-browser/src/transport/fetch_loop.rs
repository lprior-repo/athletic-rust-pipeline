use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent,
    EventResponseReceived, EventResponseReceivedExtraInfo,
};
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use futures::StreamExt;

use super::fetch_state::{FetchCapture, FetchEvent};
use super::{invocation, subscribe, MAX_CAPTURE_EVENTS};
use crate::clock::Clock;
use crate::BrowserError;

pub(super) struct FetchListeners {
    requests: EventStream<EventRequestWillBeSent>,
    responses: EventStream<EventResponseReceived>,
    finished: EventStream<EventLoadingFinished>,
    failures: EventStream<EventLoadingFailed>,
    extra: EventStream<EventResponseReceivedExtraInfo>,
    data: EventStream<EventDataReceived>,
}

pub(super) async fn subscribe_fetch(page: &Page) -> Result<FetchListeners, BrowserError> {
    Ok(FetchListeners {
        requests: subscribe(page).await?,
        responses: subscribe(page).await?,
        finished: subscribe(page).await?,
        failures: subscribe(page).await?,
        extra: subscribe(page).await?,
        data: subscribe(page).await?,
    })
}

const MAX_REQUEST_HEADERS: usize = 8;
const MAX_REQUEST_HEADER_NAME_BYTES: usize = 64;
const MAX_REQUEST_HEADER_VALUE_BYTES: usize = 1024;

pub(super) fn timeout_millis(duration: Duration) -> Result<u64, BrowserError> {
    let millis = u64::try_from(duration.as_millis()).map_err(|_| BrowserError::Protocol)?;
    if !(1..=2_147_483_647).contains(&millis) {
        return Err(BrowserError::Protocol);
    }
    Ok(millis)
}

pub(super) fn validate_request_headers(headers: &[(String, String)]) -> Result<(), BrowserError> {
    if headers.len() > MAX_REQUEST_HEADERS {
        return Err(BrowserError::Protocol);
    }
    for (name, value) in headers {
        let valid_name = !name.is_empty()
            && name.len() <= MAX_REQUEST_HEADER_NAME_BYTES
            && name.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            });
        let valid_value = value.len() <= MAX_REQUEST_HEADER_VALUE_BYTES
            && !value.bytes().any(|byte| byte == b'\r' || byte == b'\n');
        if !valid_name || !valid_value {
            return Err(BrowserError::Protocol);
        }
    }
    Ok(())
}

pub(super) async fn drive_capture(
    capture: &mut FetchCapture<'_>,
    listeners: FetchListeners,
    deadline: tokio::time::Instant,
    clock: &dyn Clock,
) -> Result<(), BrowserError> {
    let available = deadline.saturating_duration_since(clock.now_instant());
    if available.as_millis() == 0 {
        return Err(BrowserError::Timeout);
    }
    let evaluation = invocation::run(capture.page, capture.invocation);
    tokio::pin!(evaluation);
    let FetchListeners {
        mut requests,
        mut responses,
        mut finished,
        mut failures,
        mut extra,
        mut data,
    } = listeners;
    for _ in 0..MAX_CAPTURE_EVENTS {
        let remaining = deadline.saturating_duration_since(clock.now_instant());
        if remaining.is_zero() {
            return Err(BrowserError::Timeout);
        }
        let event = tokio::select! {
            biased;
            result = &mut evaluation, if capture.awaiting_evaluation() => {
                FetchEvent::Evaluation(result)
            }
            event = requests.next() => event.map_or(FetchEvent::Closed, FetchEvent::Request),
            event = responses.next() => event.map_or(FetchEvent::Closed, FetchEvent::Response),
            event = extra.next() => event.map_or(FetchEvent::Closed, FetchEvent::ExtraInfo),
            event = data.next() => event.map_or(FetchEvent::Closed, FetchEvent::Data),
            event = finished.next() => event.map_or(FetchEvent::Closed, FetchEvent::Finished),
            event = failures.next() => event.map_or(FetchEvent::Closed, FetchEvent::Failed),
            _ = tokio::time::sleep(remaining) => FetchEvent::Timer,
        };
        capture.apply(event, remaining).await?;
        match capture.complete() {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => return Err(error),
        }
    }
    Err(BrowserError::Transport)
}
