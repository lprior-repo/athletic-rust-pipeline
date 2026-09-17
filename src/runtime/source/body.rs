use crate::runtime::protocol::{FailureCode, MAX_SOURCE_RESPONSE_BYTES};
use futures::TryStreamExt;
use reqwest::{header::CONTENT_LENGTH, Response};

type BodyError = (FailureCode, String);

pub(super) async fn read_body(response: Response) -> Result<Vec<u8>, BodyError> {
    let declared = declared_length(&response)?;
    let limit = u64::try_from(MAX_SOURCE_RESPONSE_BYTES)
        .map_err(|_| transport("source byte limit overflow"))?;
    if declared.is_some_and(|size| size > limit) {
        return Err(oversize());
    }
    let body = response
        .bytes_stream()
        .map_err(|_| transport("source response body transport failed"))
        .try_fold(Vec::new(), |mut body, chunk| async move {
            let size = body
                .len()
                .checked_add(chunk.len())
                .filter(|size| *size <= MAX_SOURCE_RESPONSE_BYTES)
                .ok_or_else(oversize)?;
            if size > body.capacity() {
                let capacity = size
                    .checked_next_power_of_two()
                    .map_or(MAX_SOURCE_RESPONSE_BYTES, |size| {
                        size.min(MAX_SOURCE_RESPONSE_BYTES)
                    });
                body.try_reserve_exact(capacity - body.len())
                    .map_err(|_| transport("source body allocation failed"))?;
            }
            body.extend_from_slice(&chunk);
            Ok(body)
        })
        .await?;
    let received =
        u64::try_from(body.len()).map_err(|_| transport("source byte count overflow"))?;
    if declared.is_some_and(|size| size != received) {
        return Err(transport(
            "source body differs from declared Content-Length",
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
    use reqwest::{Body, ResponseBuilderExt};
    use std::io;

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
