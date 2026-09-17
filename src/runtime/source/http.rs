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
        Err(message) => return failure(FailureCode::Transport, None, message, true),
    };
    let status = response.status();
    let backoff = retry::retry_after(response.headers(), SystemTime::now());
    let media_type = media_type(response.headers());
    let body = match body::read_body(response).await {
        Ok(body) => body,
        Err((code, message)) => return body_failure(code, status, message, backoff),
    };
    let data = ReceiptData {
        source_url: request.semantic_url.clone(),
        status,
        media_type,
        body,
        elapsed: started.elapsed(),
    };
    match receipt(&runtime, data).await {
        Ok(receipt) => outcome(status, receipt, backoff),
        Err(message) => body_failure(FailureCode::ArtifactFailure, status, message, backoff),
    }
}

async fn send(runtime: &Runtime, request: &RequestSpec) -> Result<reqwest::Response, String> {
    let builder = match &request.body {
        Some(body) => runtime.http.post(request.url.clone()).json(body),
        None => runtime.http.get(request.url.clone()),
    };
    builder
        .send()
        .await
        .map_err(|_| "HTTP transport failure".to_owned())
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
) -> AttemptResult {
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
mod tests {
    use super::*;

    #[test]
    fn authentication_denial_is_not_a_transient_failure() {
        assert_eq!(
            status_code(StatusCode::UNAUTHORIZED),
            FailureCode::AccessDenied
        );
        assert_eq!(
            status_code(StatusCode::FORBIDDEN),
            FailureCode::AccessDenied
        );
        assert!(!retry::retryable_status(403));
        let denied = body_failure(
            FailureCode::Transport,
            StatusCode::FORBIDDEN,
            "truncated denial".to_owned(),
            Ok(Duration::ZERO),
        );
        assert!(!denied.retryable);
        let transient = body_failure(
            FailureCode::Transport,
            StatusCode::SERVICE_UNAVAILABLE,
            "truncated failure".to_owned(),
            Ok(Duration::ZERO),
        );
        assert!(transient.retryable);
    }
}
