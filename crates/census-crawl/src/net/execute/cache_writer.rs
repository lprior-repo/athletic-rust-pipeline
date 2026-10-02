use super::body_reader::read_checked_body;
use crate::net::cache::{write_archive, write_cache, CacheMeta};
use crate::net::{host_of, now_iso8601, FetchError, FetchOutcome, FetchStats};
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
    let etag = header_value(&response, reqwest::header::ETAG, meta_path)?;
    let last_modified = header_value(&response, reqwest::header::LAST_MODIFIED, meta_path)?;
    let content_type = header_value(&response, reqwest::header::CONTENT_TYPE, meta_path)?;
    let (body_vec, content_hex) = read_checked_body(response, url).await?;
    let meta = CacheMeta {
        url: url.to_string(),
        method: method.to_string(),
        status,
        content_digest: content_hex,
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        etag,
        last_modified,
        content_type,
    };
    if status == 200 {
        write_cache(body_path, meta_path, &body_vec, &meta)?;
    } else {
        write_archive(body_path, meta_path, &body_vec, &meta)?;
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
        content_type: meta.content_type,
        body: body_vec,
    })
}

fn header_value(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
    path: &Path,
) -> Result<Option<String>, FetchError> {
    response
        .headers()
        .get(name)
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|error| FetchError::Cache {
                    path: path.to_path_buf(),
                    source: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
                })
        })
        .transpose()
}

pub(super) async fn record_request_stats(stats: &Mutex<FetchStats>, url: &str, bytes: u64) {
    let mut stats = stats.lock().await;
    stats.requests = stats.requests.saturating_add(1);
    stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(bytes);
    let entry = stats.per_host.entry(host_of(url)).or_default();
    entry.requests = entry.requests.saturating_add(1);
    entry.bytes = entry.bytes.saturating_add(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::cache::{content_digest, read_cache};

    #[tokio::test]
    async fn captured_refusal_retains_headers_and_bytes_without_replacing_a_success() {
        let root = tempfile::tempdir().expect("cache directory");
        let body_path = root.path().join("source.body");
        let meta_path = root.path().join("source.meta.json");
        let stats = Mutex::new(FetchStats::default());
        for (status, raw) in [(200, b"success".as_slice()), (404, b"missing".as_slice())] {
            let response = http::Response::builder()
                .status(status)
                .header(reqwest::header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .header(reqwest::header::ETAG, "\"capture-version\"")
                .header(
                    reqwest::header::LAST_MODIFIED,
                    "Tue, 29 Sep 2026 12:00:00 GMT",
                )
                .body(raw.to_vec())
                .expect("captured response");
            let outcome = cache_and_record(
                response.into(),
                "https://example.test/source",
                "GET",
                status,
                &body_path,
                &meta_path,
                &stats,
            )
            .await
            .expect("retained response");
            assert_eq!(outcome.status, status);
            assert_eq!(outcome.body, raw);
            assert_eq!(
                outcome.content_type.as_deref(),
                Some("text/plain; charset=utf-8")
            );
            let digest = content_digest(raw);
            let saved = root
                .path()
                .join("archive/bodies")
                .join(format!("{digest}.body"));
            assert_eq!(std::fs::read(saved).expect("retained bytes"), raw);
            let captures = root.path().join("archive/captures").join(digest);
            let entry = std::fs::read_dir(captures)
                .expect("capture metadata")
                .next()
                .expect("metadata entry")
                .expect("metadata path");
            let meta: CacheMeta =
                serde_json::from_slice(&std::fs::read(entry.path()).expect("retained metadata"))
                    .expect("capture record");
            assert_eq!(meta.status, status);
            assert_eq!(meta.url, outcome.url);
            assert_eq!(meta.method, outcome.method);
            assert_eq!(meta.content_digest, outcome.content_digest);
            assert_eq!(meta.bytes, raw.len());
            assert_eq!(meta.fetched_at, outcome.fetched_at);
            assert_eq!(meta.content_type, outcome.content_type);
            assert_eq!(meta.etag.as_deref(), Some("\"capture-version\""));
            assert_eq!(
                meta.last_modified.as_deref(),
                Some("Tue, 29 Sep 2026 12:00:00 GMT")
            );
        }
        let (meta, body) = read_cache(&body_path, &meta_path)
            .expect("cache read")
            .expect("success");
        assert_eq!(meta.status, 200);
        assert_eq!(body, b"success");
        assert_eq!(meta.content_digest, content_digest(b"success"));
    }
}
