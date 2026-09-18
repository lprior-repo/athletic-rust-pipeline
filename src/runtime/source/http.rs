mod challenge;

use super::{body, request::RequestSpec, retry};
use crate::runtime::{
    protocol::{DocumentReceipt, FailureCode},
    Runtime,
};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
enum SendError {
    Session(&'static str),
    Transport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AttemptResult {
    pub(crate) receipt: Option<DocumentReceipt>,
    pub(crate) code: Option<FailureCode>,
    pub(crate) status: Option<u16>,
    pub(crate) message: String,
    pub(crate) retryable: bool,
    pub(crate) retry_after_ms: u64,
}

struct ReceiptData {
    source_url: String,
    status: StatusCode,
    media_type: String,
    body: Vec<u8>,
    elapsed: Duration,
}

pub(crate) async fn perform(runtime: Arc<Runtime>, request: &RequestSpec) -> AttemptResult {
    let started = Instant::now();
    let response = match send(&runtime, request).await {
        Ok(response) => response,
        Err(SendError::Session(message)) => {
            return failure(FailureCode::AccessDenied, None, message.to_owned(), false);
        }
        Err(SendError::Transport) => {
            return failure(
                FailureCode::Transport,
                None,
                "HTTP transport failure".to_owned(),
                true,
            );
        }
    };
    let status = response.status();
    let backoff = retry::retry_after(response.headers(), SystemTime::now());
    let media_type = media_type(response.headers());
    let header_challenge = challenge::cf_header_challenge(response.headers());
    let body = match body::read_body(response).await {
        Ok(body) => body,
        Err((code, message)) => return body_failure(code, status, message, backoff),
    };
    let challenged = header_challenge || challenge::html_body_challenge(&media_type, &body);
    let data = ReceiptData {
        source_url: request.semantic_url.clone(),
        status,
        media_type,
        body,
        elapsed: started.elapsed(),
    };
    match receipt(&runtime, data).await {
        Ok(receipt) => outcome(status, receipt, backoff, challenged),
        Err(message) => body_failure(FailureCode::ArtifactFailure, status, message, backoff),
    }
}

async fn send(runtime: &Runtime, request: &RequestSpec) -> Result<reqwest::Response, SendError> {
    let builder = match &request.body {
        Some(body) => runtime.http.post(request.url.clone()).json(body),
        None => runtime.http.get(request.url.clone()),
    };
    let builder = match runtime.config.source_session() {
        Some(session) => {
            session
                .authorize(&request.url)
                .map_err(|error| SendError::Session(error.message()))?;
            session.attach(builder)
        }
        None => builder,
    };
    builder.send().await.map_err(|_| SendError::Transport)
}

async fn receipt(runtime: &Runtime, data: ReceiptData) -> Result<DocumentReceipt, String> {
    let bytes = u64::try_from(data.body.len()).map_err(|_| "source byte count overflow")?;
    let fetched_at_unix_ms = now_ms()?;
    let elapsed_ms = millis(data.elapsed)?;
    let store = runtime.store.clone();
    let digest = runtime
        .blocking(move || Ok(store.put_bytes(&data.body)?))
        .await
        .map_err(|_| "source artifact write failed")?;
    Ok(DocumentReceipt {
        digest,
        source_url: data.source_url,
        http_status: data.status.as_u16(),
        media_type: data.media_type,
        bytes,
        fetched_at_unix_ms,
        elapsed_ms,
    })
}

fn outcome(
    status: StatusCode,
    receipt: DocumentReceipt,
    backoff: Result<Duration, &'static str>,
    challenged: bool,
) -> AttemptResult {
    if challenged && status != StatusCode::TOO_MANY_REQUESTS {
        let mut result = failure_with_receipt(
            receipt,
            status,
            FailureCode::AccessDenied,
            "source response is an access challenge",
        );
        if !status.is_success() {
            let (retry_after_ms, backoff_message, valid) = backoff_fields(backoff, status);
            result.retry_after_ms = retry_after_ms;
            if !valid {
                result.message.push_str("; ");
                result.message.push_str(&backoff_message);
            }
        }
        return result;
    }
    if status.is_success() {
        return AttemptResult {
            receipt: Some(receipt),
            code: None,
            status: Some(status.as_u16()),
            message: String::new(),
            retryable: false,
            retry_after_ms: 0,
        };
    }
    let (retry_after_ms, message, valid) = backoff_fields(backoff, status);
    AttemptResult {
        receipt: Some(receipt),
        code: Some(status_code(status)),
        status: Some(status.as_u16()),
        message,
        retryable: retry::retryable_status(status.as_u16()) && valid,
        retry_after_ms,
    }
}

fn failure_with_receipt(
    receipt: DocumentReceipt,
    status: StatusCode,
    code: FailureCode,
    message: &str,
) -> AttemptResult {
    AttemptResult {
        receipt: Some(receipt),
        code: Some(code),
        status: Some(status.as_u16()),
        message: message.to_owned(),
        retryable: false,
        retry_after_ms: 0,
    }
}

fn body_failure(
    code: FailureCode,
    status: StatusCode,
    message: String,
    backoff: Result<Duration, &'static str>,
) -> AttemptResult {
    let (retry_after_ms, backoff_message, valid) = backoff_fields(backoff, status);
    let retryable = code == FailureCode::Transport
        && valid
        && (status.is_success() || retry::retryable_status(status.as_u16()));
    let mut result = failure(code, Some(status.as_u16()), message, retryable);
    result.retry_after_ms = retry_after_ms;
    if !valid {
        result.message.push_str(&format!("; {backoff_message}"));
    }
    result
}

fn backoff_fields(
    backoff: Result<Duration, &'static str>,
    status: StatusCode,
) -> (u64, String, bool) {
    match backoff.and_then(millis) {
        Ok(delay) => (
            delay,
            format!("source returned HTTP {}", status.as_u16()),
            true,
        ),
        Err(message) => (
            0,
            format!(
                "source returned HTTP {}; {message}; operator intervention required",
                status.as_u16()
            ),
            false,
        ),
    }
}

fn media_type(headers: &HeaderMap) -> String {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map_or_else(|| "application/octet-stream".to_owned(), ToOwned::to_owned)
}

fn status_code(status: StatusCode) -> FailureCode {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => FailureCode::AccessDenied,
        StatusCode::TOO_MANY_REQUESTS => FailureCode::RateLimited,
        _ => FailureCode::HttpFailure,
    }
}

fn failure(
    code: FailureCode,
    status: Option<u16>,
    message: String,
    retryable: bool,
) -> AttemptResult {
    AttemptResult {
        receipt: None,
        code: Some(code),
        status,
        message,
        retryable,
        retry_after_ms: 0,
    }
}

fn millis(value: Duration) -> Result<u64, &'static str> {
    u64::try_from(value.as_millis()).map_err(|_| "duration exceeds supported milliseconds")
}

fn now_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock predates Unix epoch")?;
    millis(elapsed).map_err(str::to_owned)
}

#[cfg(test)]
mod tests;
