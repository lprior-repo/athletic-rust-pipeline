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
    let response_url = Some(response.url().as_str().to_owned());
    let (body_vec, content_hex) = read_checked_body(response, url).await?;
    let meta = CacheMeta {
        url: url.to_string(),
        response_url,
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
    let downloaded = u64::try_from(body_vec.len()).map_or(u64::MAX, |value| value);
    record_request_stats(stats, url, downloaded).await;
    Ok(FetchOutcome {
        url: meta.url,
        response_url: meta.response_url,
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

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    async fn acquired_response(status: u16, raw: &[u8]) -> TestResult<(String, reqwest::Response)> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/source", listener.local_addr()?);
        let server = async {
            let (mut socket, _) = listener.accept().await?;
            let mut request = [0_u8; 4096];
            if socket.read(&mut request).await? == 0 {
                return Err("missing HTTP fixture request".into());
            }
            let headers = format!(
                "HTTP/1.1 {status} Capture\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\nETag: \"capture-version\"\r\nLast-Modified: Tue, 29 Sep 2026 12:00:00 GMT\r\nConnection: close\r\n\r\n",
                raw.len()
            );
            socket.write_all(headers.as_bytes()).await?;
            socket.write_all(raw).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        };
        let client =
            async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(reqwest::get(&url).await?) };
        let ((), response) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::try_join!(server, client)
        })
        .await??;
        Ok((url, response))
    }

    fn assert_archived(root: &Path, outcome: &FetchOutcome) -> TestResult {
        let digest = content_digest(&outcome.body);
        let saved = root.join("archive/bodies").join(format!("{digest}.body"));
        check!(eq; std::fs::read(saved)?, outcome.body);
        let captures = root.join("archive/captures").join(digest);
        let entry = std::fs::read_dir(captures)?
            .next()
            .ok_or("capture metadata missing")??;
        let meta: CacheMeta = serde_json::from_slice(&std::fs::read(entry.path())?)?;
        check!(eq; meta.status, outcome.status);
        check!(eq; meta.url, outcome.url);
        check!(eq; meta.response_url, outcome.response_url);
        check!(eq; meta.method, outcome.method);
        check!(eq; meta.content_digest, outcome.content_digest);
        check!(eq; meta.bytes, outcome.body.len());
        check!(eq; meta.fetched_at, outcome.fetched_at);
        check!(eq; meta.content_type, outcome.content_type);
        check!(eq; meta.etag.as_deref(), Some("\"capture-version\""));
        check!(eq;
            meta.last_modified.as_deref(),
            Some("Tue, 29 Sep 2026 12:00:00 GMT")
        );
        Ok(())
    }

    #[test]
    fn captured_refusal_retains_headers_and_bytes_without_replacing_a_success() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let root = tempfile::tempdir()?;
                let body_path = root.path().join("source.body");
                let meta_path = root.path().join("source.meta.json");
                let stats = Mutex::new(FetchStats::default());
                for (status, raw) in [(200, b"success".as_slice()), (404, b"missing".as_slice())] {
                    let (requested, response) = acquired_response(status, raw).await?;
                    let response_url = response.url().as_str().to_string();
                    let outcome = cache_and_record(
                        response, &requested, "GET", status, &body_path, &meta_path, &stats,
                    )
                    .await?;
                    check!(eq; outcome.status, status);
                    check!(eq; outcome.body, raw);
                    check!(eq; outcome.response_url.as_deref(), Some(response_url.as_str()));
                    check!(eq;
                        outcome.content_type.as_deref(),
                        Some("text/plain; charset=utf-8")
                    );
                    assert_archived(root.path(), &outcome)?;
                }
                let (meta, body) =
                    read_cache(&body_path, &meta_path)?.ok_or("successful capture missing")?;
                check!(eq; meta.status, 200);
                check!(eq; body, b"success");
                check!(eq; meta.content_digest, content_digest(b"success"));
                Ok(())
            })
    }
}
