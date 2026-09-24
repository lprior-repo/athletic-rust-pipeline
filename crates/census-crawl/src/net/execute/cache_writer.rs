//! Shared cache-write and stats-update logic for response handlers.
//!
//! `process_ok` and `handle_404` both need to read a response body, write it to cache,
//! update stats, and construct a `FetchOutcome`. This module extracts that common path.

use super::body_reader::read_checked_body;
use crate::net::cache::{write_cache, CacheMeta};
use crate::net::{now_iso8601, FetchOutcome, FetchStats};
use std::path::Path;
use tokio::sync::Mutex;

/// Cache-and-record: read the body, write cache, update stats, return outcome.
///
/// This is the shared path for `process_ok` and `handle_404`.
pub(super) async fn cache_and_record(
    response: reqwest::Response,
    url: &str,
    method: &str,
    status: u16,
    body_path: &Path,
    meta_path: &Path,
    stats: &Mutex<FetchStats>,
) -> Result<FetchOutcome, crate::net::FetchError> {
    let (body_vec, key_prefix, content_hex) = read_checked_body(response, url).await?;
    let meta = CacheMeta {
        url: url.to_string(),
        method: method.to_string(),
        status,
        key_prefix: key_prefix.clone(),
        content_digest: content_hex,
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        etag: None,
        last_modified: None,
        content_type: None,
    };
    write_cache(body_path, meta_path, &body_vec, &meta)?;
    record_request_stats(stats, url).await;
    {
        let mut stats = stats.lock().await;
        let downloaded = u64::try_from(body_vec.len()).unwrap_or(u64::MAX);
        stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(downloaded);
    }
    Ok(FetchOutcome {
        url: url.to_string(),
        method: method.to_string(),
        status,
        sha256: key_prefix,
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        from_cache: false,
        content_type: None,
        body: body_vec,
    })
}

/// Record request counts in the stats.
pub(super) async fn record_request_stats(stats: &Mutex<FetchStats>, host: &str) {
    let mut stats = stats.lock().await;
    stats.requests = stats.requests.saturating_add(1);
    let per_host_entry = stats.per_host.entry(host.to_string()).or_insert(0);
    *per_host_entry = per_host_entry.saturating_add(1);
}
