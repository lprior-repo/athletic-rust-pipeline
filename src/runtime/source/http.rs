use super::retry;
use crate::runtime::{
    protocol::{DocumentReceipt, FailureCode},
    Runtime,
};
use athleticnet_browser::protocol::RankingPageObservation;
use athleticnet_browser::request::{RequestAction, RequestSpec};
use athleticnet_browser::retry::retry_after_now;
use athleticnet_browser::{BrowserError, BrowserFailure, BrowserOutcome};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

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
    rankings: Option<RankingPageObservation>,
}

pub(crate) async fn perform(runtime: Arc<Runtime>, request: &RequestSpec) -> AttemptResult {
    let clock = runtime.clock();
    let started = clock.now_instant();
    let browser = match runtime.ensure_browser().await {
        Ok(browser) => browser,
        Err(_) => {
            return failure(
                FailureCode::Transport,
                None,
                "browser startup failed".to_owned(),
                false,
            );
        }
    };
    let capture = match browser.fetch(request.clone()).await {
        BrowserOutcome::Captured(capture) => *capture,
        BrowserOutcome::Failed(failure) => return browser_failure(failure),
    };
    // The challenge verdict is the transport's own classification, read here rather than recomposed
    // from the same headers and body: the gate the transport applied and this receipt are then one
    // decision instead of two readings of one response that can disagree.
    let challenged = capture.challenge;
    let response = capture.response;
    let status = response.status;
    let backoff = retry_after_now(clock.as_ref(), &response.headers);
    let media_type = media_type(&response.headers);
    let data = ReceiptData {
        source_url: request.semantic_url.clone(),
        status,
        media_type,
        body: response.body,
        elapsed: started.elapsed(),
        rankings: response.rankings,
    };
    match receipt(&runtime, data, Some(&request.action)).await {
        Ok(receipt) => outcome(status, receipt, backoff, challenged),
        Err(message) => body_failure(FailureCode::ArtifactFailure, status, message, backoff),
    }
}

fn browser_failure(stopped: BrowserFailure) -> AttemptResult {
    // The failure code stays this crate's vocabulary; whether another invocation is worth making is
    // the transport's verdict, so the report and the transport cannot drift apart.
    let code = match stopped.error {
        BrowserError::HumanRequired | BrowserError::Unavailable | BrowserError::TaskPanicked => {
            FailureCode::BrowserUnavailable
        }
        BrowserError::PayloadLimit => FailureCode::PayloadLimit,
        BrowserError::Redirect => FailureCode::HttpFailure,
        BrowserError::Protocol => FailureCode::MalformedResponse,
        BrowserError::Shutdown | BrowserError::Transport | BrowserError::Timeout => {
            FailureCode::Transport
        }
    };
    failure(
        code,
        None,
        stopped.error.to_string(),
        stopped.verdict.retryable(),
    )
}

async fn receipt(
    runtime: &Runtime,
    data: ReceiptData,
    request_action: Option<&RequestAction>,
) -> Result<DocumentReceipt, String> {
    let bytes = u64::try_from(data.body.len()).map_err(|_| "source byte count overflow")?;
    // Receipt timestamps are wall clock by definition: they are read back by operators and by the
    // export bundle, so a monotonic reading would be meaningless.
    let fetched_at_unix_ms = runtime
        .clock()
        .now_unix_ms()
        .map_err(|error| error.to_string())?;
    let elapsed_ms = millis(data.elapsed)?;
    let store = runtime.store.clone();
    let digest = runtime
        .blocking(move || Ok(store.put_bytes(&data.body)?))
        .await
        .map_err(|_| "source artifact write failed")?;

    // Rankings capture is passed through verbatim from BrowserResponse
    // Parsing happens in the collection/domain layer
    let rankings = match request_action {
        Some(RequestAction::Rankings(_)) => data.rankings,
        _ => None,
    };

    Ok(DocumentReceipt {
        digest,
        source_url: data.source_url,
        http_status: data.status.as_u16(),
        media_type: data.media_type,
        bytes,
        fetched_at_unix_ms,
        elapsed_ms,
        rankings,
    })
}

fn outcome(
    status: StatusCode,
    receipt: DocumentReceipt,
    backoff: Result<Duration, &'static str>,
    challenged: bool,
) -> AttemptResult {
    if challenged {
        let mut result = failure_with_receipt(
            receipt,
            status,
            FailureCode::BrowserChallenge,
            "source browser challenge; awaiting profile recovery through Restate",
        );
        let (retry_after_ms, message, valid) = backoff_fields(backoff, status);
        result.retry_after_ms = retry_after_ms;
        result.retryable = valid;
        if !valid {
            result.message.push_str("; ");
            result.message.push_str(&message);
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

#[cfg(test)]
mod tests;
