use super::body_reader::read_checked_body;
use crate::net::cache::{write_cache, CacheMeta};
use crate::net::{host_of, now_iso8601, FetchOutcome, FetchStats};
use std::path::Path;
use tokio::sync::Mutex;

pub(super) async fn cache_and_record(
    response: reqwest::Response,
    url: &str,
    method: &str,
    status: u16,
    body_path: &Path,
    meta_path: &Path,
    stats: &Mutex<FetchStats>,
) -> Result<FetchOutcome, crate::net::FetchError> {
    let (body_vec, content_hex) = read_checked_body(response, url).await?;
    let meta = CacheMeta {
        url: url.to_string(),
        method: method.to_string(),
        status,
        content_digest: content_hex,
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        etag: None,
        last_modified: None,
        content_type: None,
    };
    if status == 200 {
        write_cache(body_path, meta_path, &body_vec, &meta)?;
    }
    let downloaded = u64::try_from(body_vec.len()).unwrap_or(u64::MAX);
    record_request_stats(stats, url, downloaded).await;
    Ok(FetchOutcome {
        url: meta.url,
        method: meta.method,
        status,
        content_digest: meta.content_digest,
        bytes: body_vec.len(),
        fetched_at: meta.fetched_at,
        from_cache: false,
        content_type: None,
        body: body_vec,
    })
}

pub(super) async fn record_request_stats(stats: &Mutex<FetchStats>, url: &str, bytes: u64) {
    let mut stats = stats.lock().await;
    stats.requests = stats.requests.saturating_add(1);
    stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(bytes);
    let entry = stats.per_host.entry(host_of(url)).or_default();
    entry.requests = entry.requests.saturating_add(1);
    entry.bytes = entry.bytes.saturating_add(bytes);
}
