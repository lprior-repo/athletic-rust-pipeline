use crate::runtime::protocol::{FailureCode, MAX_SOURCE_RESPONSE_BYTES};
use futures::StreamExt;
use reqwest::{header::CONTENT_LENGTH, Body, Response, ResponseBuilderExt};
use std::{
    io,
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
};

type BodyError = (FailureCode, String);
const COMPLETE: u8 = 0;
const OVERSIZE: u8 = 1;
const TRANSPORT: u8 = 2;

#[derive(Default)]
struct StreamState {
    bytes: AtomicUsize,
    failure: AtomicU8,
}

// Spider's private process-wide cap must not silently narrow our public bound.
// The worker never mutates this environment after initialization.
pub(crate) fn validate_environment() -> anyhow::Result<()> {
    match std::env::var("SPIDER_MAX_SIZE_BYTES") {
        Ok(raw) => {
            let limit = raw.parse::<usize>()?;
            if limit != 0 && limit < MAX_SOURCE_RESPONSE_BYTES {
                anyhow::bail!("SPIDER_MAX_SIZE_BYTES must be zero or at least 32 MiB");
            }
            Ok(())
        }
        Err(std::env::VarError::NotPresent) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn read_body(response: Response) -> Result<Vec<u8>, BodyError> {
    let declared = declared_length(&response)?;
    if declared.is_some_and(|size| size > MAX_SOURCE_RESPONSE_BYTES as u64) {
        return Err(oversize());
    }
    let target = response.url().to_string();
    let state = Arc::new(StreamState::default());
    let response = bounded_response(response, state.clone())?;
    // This Spider API only consumes an existing response: no requests or retries.
    let page = spider::utils::handle_response_bytes(response, &target, false).await;
    match state.failure.load(Ordering::Relaxed) {
        OVERSIZE => return Err(oversize()),
        TRANSPORT => return Err(transport("source response body transport failed")),
        COMPLETE => {}
        _ => return Err(transport("invalid source body accounting state")),
    }
    if page.content_truncated {
        return Err(transport("Spider reported an incomplete source response"));
    }
    let received = state.bytes.load(Ordering::Relaxed);
    let received_u64 =
        u64::try_from(received).map_err(|_| transport("source byte count overflow"))?;
    if declared.is_some_and(|size| size != received_u64) {
        return Err(transport(
            "source body differs from declared Content-Length",
        ));
    }
    let body = page
        .content
        .ok_or_else(|| transport("Spider returned no complete source body"))?;
    if body.len() != received {
        return Err(transport(
            "Spider did not retain every received source byte",
        ));
    }
    Ok(body)
}

fn declared_length(response: &Response) -> Result<Option<u64>, BodyError> {
    match response.headers().get(CONTENT_LENGTH) {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse().ok())
            .map(Some)
            .ok_or_else(|| transport("source Content-Length is invalid")),
        None => Ok(response.content_length()),
    }
}

fn bounded_response(response: Response, state: Arc<StreamState>) -> Result<Response, BodyError> {
    let status = response.status();
    let version = response.version();
    let headers = response.headers().clone();
    let url = response.url().clone();
    let stream = response.bytes_stream().map(move |item| {
        let chunk = match item {
            Ok(chunk) => chunk,
            Err(_) => {
                state.failure.store(TRANSPORT, Ordering::Relaxed);
                return Err(io::Error::other("source body transport failure"));
            }
        };
        let Some(size) = state
            .bytes
            .load(Ordering::Relaxed)
            .checked_add(chunk.len())
            .filter(|size| *size <= MAX_SOURCE_RESPONSE_BYTES)
        else {
            state.failure.store(OVERSIZE, Ordering::Relaxed);
            return Err(io::Error::other("source response exceeds 32 MiB"));
        };
        state.bytes.store(size, Ordering::Relaxed);
        Ok(chunk)
    });
    let mut wrapped = http::Response::builder()
        .status(status)
        .version(version)
        .url(url)
        .body(Body::wrap_stream(stream))
        .map_err(|_| transport("adapting source response for Spider failed"))?;
    *wrapped.headers_mut() = headers;
    Ok(wrapped.into())
}

fn oversize() -> BodyError {
    (
        FailureCode::PayloadLimit,
        "source response exceeds 32 MiB".to_owned(),
    )
}

fn transport(message: &str) -> BodyError {
    (FailureCode::Transport, message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(body: Body, length: Option<u64>) -> Response {
        let mut response = http::Response::builder()
            .status(200)
            .url("http://127.0.0.1/source".parse().expect("fixture URL"));
        if let Some(length) = length {
            response = response.header(CONTENT_LENGTH, length);
        }
        response.body(body).expect("fixture response").into()
    }

    #[tokio::test]
    async fn retains_exact_body_and_rejects_declared_truncation() {
        let source = b"<html>synthetic &amp; body</html>";
        let body = read_body(response(Body::from(source.as_slice()), None))
            .await
            .expect("complete body");
        assert_eq!(body, source);
        let error = read_body(response(Body::from("short"), Some(10)))
            .await
            .expect_err("partial body");
        assert_eq!(error.0, FailureCode::Transport);
    }

    #[tokio::test]
    async fn rejects_chunked_oversize_and_stream_failure() {
        let chunks = futures::stream::iter([
            Ok::<_, io::Error>(vec![b'x'; MAX_SOURCE_RESPONSE_BYTES]),
            Ok(vec![b'y']),
        ]);
        let error = read_body(response(Body::wrap_stream(chunks), None))
            .await
            .expect_err("oversized body");
        assert_eq!(error.0, FailureCode::PayloadLimit);
        let chunks = futures::stream::iter([
            Ok(vec![b'x']),
            Err(io::Error::other("synthetic disconnected body")),
        ]);
        let error = read_body(response(Body::wrap_stream(chunks), None))
            .await
            .expect_err("broken body");
        assert_eq!(error.0, FailureCode::Transport);
    }
}
