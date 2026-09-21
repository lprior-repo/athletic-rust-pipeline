use super::{
    input,
    model::{parse_response, Attempt, ChatRequest},
    terminal, Runtime,
};
use crate::runtime::{
    protocol::{FailureCode, ReviewInput, MAX_REVIEW_RESPONSE_BYTES},
    source::retry::{retry_after_now, retryable_status},
};
use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use reqwest::{header::CONTENT_TYPE, StatusCode};
use restate_sdk::prelude::*;
use std::time::Duration;

// SDK retry policy starts at one second. Retry-After values beyond that
// cannot be represented by the SDK policy and are handled as a durable
// post-operation cooldown instead of an application retry schedule.
const SDK_INITIAL_DELAY: Duration = Duration::from_secs(1);

pub async fn request_once(
    runtime: &Runtime,
    endpoint: &str,
    request: &ChatRequest,
    input: &ReviewInput,
) -> Result<Attempt, HandlerError> {
    let clock = runtime.clock();
    let started = clock.now_instant();
    let response = match runtime.http.post(endpoint).json(request).send().await {
        Ok(response) => response,
        Err(error) => return Ok(transport_failure(error)),
    };
    let status = response.status();
    let retry_after = retry_after_now(clock.as_ref(), response.headers());
    let media_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map_or_else(|| "application/octet-stream".to_owned(), ToOwned::to_owned);
    let body = match bounded_body(response).await {
        Ok(body) => body,
        Err(error) => return Ok(body_failure(error, status, retry_after)),
    };
    let parsed = status.is_success().then(|| parse_response(&body, input));
    let receipt = store_response(runtime, endpoint, status, &media_type, body, started)
        .await
        .map_err(terminal)?;
    if !status.is_success() {
        return Ok(http_failure(status, receipt, retry_after));
    }
    match parsed {
        Some(Ok(verdict)) => Ok(Attempt::Success { verdict, receipt }),
        Some(Err(error)) => Ok(Attempt::Failure {
            code: FailureCode::MalformedResponse,
            message: error.to_string(),
            status: Some(status.as_u16()),
            receipt: Some(receipt),
            retryable: false,
            retry_after_ms: 0,
        }),
        None => Ok(Attempt::Failure {
            code: FailureCode::MalformedResponse,
            message: "local model response was empty".to_owned(),
            status: Some(status.as_u16()),
            receipt: Some(receipt),
            retryable: false,
            retry_after_ms: 0,
        }),
    }
}

async fn bounded_body(response: reqwest::Response) -> Result<Vec<u8>> {
    let cap = u64::try_from(MAX_REVIEW_RESPONSE_BYTES)?;
    if response.content_length().is_some_and(|size| size > cap) {
        return Err(anyhow!("local model response exceeds 32 KiB"));
    }
    response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut body, chunk| async move {
            let size = body
                .len()
                .checked_add(chunk.len())
                .ok_or_else(|| anyhow!("local model response size overflow"))?;
            if size > MAX_REVIEW_RESPONSE_BYTES {
                return Err(anyhow!("local model response exceeds 32 KiB"));
            }
            body.try_reserve(chunk.len())
                .map_err(|error| anyhow!("allocating local model response: {error}"))?;
            body.extend_from_slice(&chunk);
            Ok(body)
        })
        .await
}

fn retry_after_fields(
    retry_after: Result<Duration, &'static str>,
) -> (bool, u64, Option<&'static str>) {
    match retry_after {
        Ok(delay) => {
            let delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
            if delay <= SDK_INITIAL_DELAY {
                (true, delay_ms, None)
            } else {
                (
                    false,
                    delay_ms,
                    Some("Retry-After exceeds SDK guaranteed delay; durable cooldown required"),
                )
            }
        }
        Err(reason) => (false, 0, Some(reason)),
    }
}

fn body_failure(
    error: anyhow::Error,
    status: StatusCode,
    retry_after: Result<Duration, &'static str>,
) -> Attempt {
    let retryable_body = error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|value| value.is_timeout() || value.is_connect() || value.is_body());
    let transient_status = status.is_success() || retryable_status(status.as_u16());
    let (policy_retryable, retry_after_ms, limitation) = retry_after_fields(retry_after);
    let retryable = retryable_body && transient_status && policy_retryable;
    let code = if retryable_body {
        FailureCode::Transport
    } else {
        FailureCode::PayloadLimit
    };
    let message = limitation.map_or_else(
        || error.to_string(),
        |reason| format!("{}; {reason}; operator intervention required", error),
    );
    Attempt::Failure {
        code,
        message,
        status: Some(status.as_u16()),
        receipt: None,
        retryable,
        retry_after_ms,
    }
}

async fn store_response(
    runtime: &Runtime,
    endpoint: &str,
    status: StatusCode,
    media_type: &str,
    body: Vec<u8>,
    started: tokio::time::Instant,
) -> Result<crate::runtime::protocol::DocumentReceipt> {
    let size = body.len();
    let store = runtime.store.clone();
    let digest = runtime
        .blocking(move || Ok(store.put_bytes(&body)?))
        .await?;
    let elapsed =
        u64::try_from(started.elapsed().as_millis()).map_err(|error| anyhow!(error.to_string()))?;
    input::receipt(
        &runtime.clock(),
        digest,
        endpoint.to_owned(),
        status.as_u16(),
        media_type,
        size,
        elapsed,
    )
}

fn transport_failure(error: reqwest::Error) -> Attempt {
    let retryable = error.is_timeout() || error.is_connect() || error.is_body();
    Attempt::Failure {
        code: FailureCode::Transport,
        message: "local model transport failed".to_owned(),
        status: None,
        receipt: None,
        retryable,
        retry_after_ms: 0,
    }
}

fn http_failure(
    status: StatusCode,
    receipt: crate::runtime::protocol::DocumentReceipt,
    retry_after: Result<Duration, &'static str>,
) -> Attempt {
    let (code, transient) = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => (FailureCode::AccessDenied, false),
        StatusCode::TOO_MANY_REQUESTS => (FailureCode::RateLimited, true),
        value if value.is_server_error() => (FailureCode::HttpFailure, true),
        _ => (FailureCode::HttpFailure, false),
    };
    let (policy_retryable, retry_after_ms, limitation) = retry_after_fields(retry_after);
    let retryable = transient && policy_retryable;
    let message = limitation.map_or_else(
        || format!("local model returned HTTP {}", status.as_u16()),
        |reason| {
            format!(
                "local model returned HTTP {}; {reason}; operator intervention required",
                status.as_u16()
            )
        },
    );
    Attempt::Failure {
        code,
        message,
        status: Some(status.as_u16()),
        receipt: Some(receipt),
        retryable,
        retry_after_ms,
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
