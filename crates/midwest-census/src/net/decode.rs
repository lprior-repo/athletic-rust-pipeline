//! Response decoding: size guard, hashing, cache write and outcome construction.

use super::cache::{sha256_prefix16, write_cache, CacheMeta};
use super::{now_iso8601, FetchError, FetchOptions, FetchOutcome, FetchStats, MAX_BODY_BYTES};
use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::warn;

/// Process a successful response: check size, read body, hash, cache, update stats.
///
/// The eight parameters are the request's own coordinates plus the two cache paths it writes; a
/// struct would only move the same list one level up.
#[allow(clippy::too_many_arguments)]
pub(super) async fn process_response(
    response: reqwest::Response,
    url: &str,
    method: &str,
    host: &str,
    body_path: &Path,
    meta_path: &Path,
    options: &FetchOptions,
    stats: &mut FetchStats,
) -> Result<FetchOutcome> {
    let status = response.status().as_u16();
    let (etag, last_modified, content_type) = header_strings(response.headers());

    stats.requests = stats.requests.saturating_add(1);
    let per_host_entry = stats.per_host.entry(host.to_string()).or_insert(0);
    *per_host_entry = per_host_entry.saturating_add(1);

    if status == 304 {
        // This branch should not be reached here (304 is handled above), but guard defensively.
        bail!("unexpected 304 in process_response");
    }

    let (body_vec, sha256) = read_checked_body(response, url).await?;
    let meta = CacheMeta {
        url: url.to_string(),
        method: method.to_string(),
        status,
        sha256: sha256.clone(),
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        etag,
        last_modified,
        content_type: content_type.clone(),
    };
    write_cache(body_path, meta_path, &body_vec, &meta)?;
    // `usize` -> `u64` for the byte counter, saturating where the value cannot fit.
    let downloaded = u64::try_from(body_vec.len()).unwrap_or(u64::MAX);
    stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(downloaded);
    if status >= 400 && !(status == 404 && options.allow_not_found) {
        stats.errors = stats.errors.saturating_add(1);
        warn!(status, url, "non-success response");
    }
    Ok(FetchOutcome {
        url: url.to_string(),
        method: method.to_string(),
        status,
        sha256,
        bytes: body_vec.len(),
        fetched_at: meta.fetched_at,
        from_cache: false,
        content_type,
        body: body_vec,
    })
}

/// The three response headers the cache keeps, as owned strings.
fn header_strings(
    headers: &reqwest::header::HeaderMap,
) -> (Option<String>, Option<String>, Option<String>) {
    let header = |name: reqwest::header::HeaderName| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    (
        header(reqwest::header::ETAG),
        header(reqwest::header::LAST_MODIFIED),
        header(reqwest::header::CONTENT_TYPE),
    )
}

/// Read the response body inside the size cap, and hash what was read.
async fn read_checked_body(response: reqwest::Response, url: &str) -> Result<(Vec<u8>, String)> {
    let declared = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    if declared.map(|len| len > MAX_BODY_BYTES).unwrap_or(false) {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        }
        .into());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|source| FetchError::Transport {
            url: url.to_string(),
            source,
        })?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        }
        .into());
    }
    let body = bytes.to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&body);
    Ok((body, sha256_prefix16(hasher)))
}
