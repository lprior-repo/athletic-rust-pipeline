use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventLoadingFailed, EventLoadingFinished, EventRequestWillBeSent, EventResponseReceived,
    EventResponseReceivedExtraInfo,
};
use chromiumoxide::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use futures::StreamExt;

use super::fetch_state::{FetchCapture, FetchEvent};
use super::{fail, request_method, script, subscribe, FetchArguments, MAX_CAPTURE_EVENTS};
use crate::clock::Clock;
use crate::protocol::MAX_SOURCE_RESPONSE_BYTES;
use crate::request::RequestSpec;
use crate::BrowserError;

pub(super) struct FetchListeners {
    requests: EventStream<EventRequestWillBeSent>,
    responses: EventStream<EventResponseReceived>,
    finished: EventStream<EventLoadingFinished>,
    failures: EventStream<EventLoadingFailed>,
    extra: EventStream<EventResponseReceivedExtraInfo>,
}

pub(super) async fn subscribe_fetch(page: &Page) -> Result<FetchListeners, BrowserError> {
    Ok(FetchListeners {
        requests: subscribe(page).await?,
        responses: subscribe(page).await?,
        finished: subscribe(page).await?,
        failures: subscribe(page).await?,
        extra: subscribe(page).await?,
    })
}

const MAX_REQUEST_HEADERS: usize = 8;
const MAX_REQUEST_HEADER_NAME_BYTES: usize = 64;
const MAX_REQUEST_HEADER_VALUE_BYTES: usize = 1024;

fn validate_request_headers(headers: &[(String, String)]) -> Result<(), BrowserError> {
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

fn fetch_call(
    request: &RequestSpec,
    request_body: Option<&str>,
    request_timeout: Duration,
) -> Result<CallFunctionOnParams, BrowserError> {
    validate_request_headers(&request.headers)?;
    let arguments = FetchArguments {
        url: request.url.as_str(),
        method: request_method(request),
        body: request_body,
        timeout_ms: u64::try_from(request_timeout.as_millis())
            .map_err(|_| BrowserError::Protocol)?,
        max_body: MAX_SOURCE_RESPONSE_BYTES,
        headers: request.headers.as_slice(),
    };
    let value = serde_json::to_value(arguments).map_err(|_| BrowserError::Protocol)?;
    CallFunctionOnParams::builder()
        .function_declaration(script::FETCH_FUNCTION)
        .argument(CallArgument::builder().value(value).build())
        .return_by_value(true)
        .await_promise(true)
        .build()
        .map_err(|_| BrowserError::Protocol)
}

pub(super) async fn drive_capture(
    capture: &mut FetchCapture<'_>,
    listeners: FetchListeners,
    request_timeout: Duration,
    clock: &dyn Clock,
) -> Result<(), BrowserError> {
    let call = fetch_call(capture.request, capture.request_body, request_timeout)?;
    let evaluation = capture.page.evaluate_function(call);
    tokio::pin!(evaluation);
    let now = clock.now_instant();
    let deadline = now.checked_add(request_timeout).map_or(now, |value| value);
    let FetchListeners {
        mut requests,
        mut responses,
        mut finished,
        mut failures,
        mut extra,
    } = listeners;
    for _ in 0..MAX_CAPTURE_EVENTS {
        let remaining = deadline.saturating_duration_since(clock.now_instant());
        if remaining.is_zero() {
            return Err(fail(capture.page, capture.gate, BrowserError::Timeout).await);
        }
        let event = tokio::select! {
            biased;
            result = &mut evaluation, if capture.awaiting_evaluation() => {
                FetchEvent::Evaluation(match result {
                    Ok(value) => value.into_value().map_err(|_| BrowserError::Protocol),
                    Err(_) => Err(BrowserError::Transport),
                })
            }
            event = requests.next() => event.map_or(FetchEvent::Closed, FetchEvent::Request),
            event = responses.next() => event.map_or(FetchEvent::Closed, FetchEvent::Response),
            event = extra.next() => event.map_or(FetchEvent::Closed, FetchEvent::ExtraInfo),
            event = finished.next() => event.map_or(FetchEvent::Closed, FetchEvent::Finished),
            event = failures.next() => event.map_or(FetchEvent::Closed, FetchEvent::Failed),
            _ = tokio::time::sleep(remaining) => FetchEvent::Timer,
        };
        if let Err(error) = capture.apply(event, remaining).await {
            return Err(fail(capture.page, capture.gate, error).await);
        }
        if capture.complete()? {
            break;
        }
    }
    Ok(())
}
