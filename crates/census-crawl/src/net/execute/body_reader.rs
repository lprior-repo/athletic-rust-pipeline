use crate::net::FetchError;
use crate::net::MAX_BODY_BYTES;
use futures::StreamExt;
use sha2::{Digest, Sha256};

pub(super) async fn read_checked_body(
    response: reqwest::Response,
    url: &str,
) -> Result<(Vec<u8>, String), FetchError> {
    let declared = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    if declared.is_some_and(|len| len > MAX_BODY_BYTES) {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        });
    }
    let mut body = Vec::with_capacity(declared.map_or(8 * 1024, |value| value));
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) =
        stream
            .next()
            .await
            .transpose()
            .map_err(|source| FetchError::Transport {
                url: url.to_string(),
                source,
            })?
    {
        let chunk_len = chunk.len();
        if body.len().saturating_add(chunk_len) > MAX_BODY_BYTES {
            return Err(FetchError::TooLarge {
                url: url.to_string(),
            });
        }
        hasher.update(&chunk);
        body.extend_from_slice(&chunk);
    }
    Ok((body, format!("{:x}", hasher.finalize())))
}
