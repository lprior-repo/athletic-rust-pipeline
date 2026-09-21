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
    let headers = response.headers().clone();
    let etag = headers
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let last_modified = headers
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    stats.requests = stats.requests.saturating_add(1);
    let per_host_entry = stats.per_host.entry(host.to_string()).or_insert(0);
    *per_host_entry = per_host_entry.saturating_add(1);

    if status == 304 {
        // This branch should not be reached here (304 is handled above), but guard defensively.
        bail!("unexpected 304 in process_response");
    }

    let declared = headers
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
    let body_vec = bytes.to_vec();
    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(&body_vec);
        sha256_prefix16(hasher)
    };
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
