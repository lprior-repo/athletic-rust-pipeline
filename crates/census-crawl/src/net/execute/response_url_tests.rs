use crate::net::cache::{content_digest, read_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod transport_policy;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const BODY: &[u8] = b"redirected public evidence";

const OVERSIZED_RETRY_AFTER: &str = "18446744073709551615";

async fn request_headers(socket: &mut TcpStream) -> TestResult<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 512];
    for _ in 0..32 {
        let length = socket.read(&mut buffer).await?;
        if length == 0 {
            return Err("request ended before headers".into());
        }
        bytes.extend_from_slice(buffer.get(..length).ok_or("read boundary")?);
        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            return Ok(String::from_utf8(bytes)?);
        }
    }
    Err("request headers exceed fixture bound".into())
}

async fn serve(listener: TcpListener) -> TestResult {
    for _ in 0..4 {
        let (mut socket, _) = listener.accept().await?;
        let request = request_headers(&mut socket).await?;
        let response = match request.split_whitespace().nth(1).ok_or("missing request path")? {
            "/start" => "HTTP/1.1 302 Found\r\nLocation: /finish\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            "/finish" if request.to_ascii_lowercase().contains("if-none-match: \"capture-v1\"") =>
                "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            "/finish" => format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"capture-v1\"\r\nConnection: close\r\n\r\n{}",
                BODY.len(), std::str::from_utf8(BODY)?
            ),
            path => return Err(format!("unexpected request path {path}").into()),
        };
        socket.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

async fn serve_rate_limited(listener: TcpListener) -> TestResult {
    let (mut socket, _) = listener.accept().await?;
    let _request = request_headers(&mut socket).await?;
    let response = format!(
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: {OVERSIZED_RETRY_AFTER}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nno"
    );
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn serve_date_limited(listener: TcpListener, retry_after: String) -> TestResult {
    let (mut socket, _) = listener.accept().await?;
    let _request = request_headers(&mut socket).await?;
    let response = format!(
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: {retry_after}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nno"
    );
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

fn assert_capture(outcome: &FetchOutcome, requested: &str, final_url: &str) -> TestResult {
    check!(eq; outcome.url, requested);
    check!(eq; outcome.response_url.as_deref(), Some(final_url));
    check!(eq; outcome.body, BODY);
    check!(eq; outcome.content_digest, content_digest(BODY));
    check!(eq; outcome.status, 200);
    Ok(())
}

fn assert_archive(cache: &Path, meta: &CacheMeta) -> TestResult {
    check!(eq;
        std::fs::read(
            cache
                .join("archive/bodies")
                .join(format!("{}.body", meta.content_digest))
        )?,
        BODY
    );
    let directory = cache.join("archive/captures").join(&meta.content_digest);
    let mut captures = std::fs::read_dir(directory)?;
    let entry = captures
        .next()
        .ok_or("missing physical capture metadata")??;
    let archived: CacheMeta = serde_json::from_slice(&std::fs::read(entry.path())?)?;
    check!(eq; &archived, meta);
    check!(captures.next().is_none());
    Ok(())
}

async fn acquire(
    fetcher: &Fetcher,
    cache: &Path,
    requested: &str,
    final_url: &str,
) -> TestResult<String> {
    let first = fetcher.get(requested, &FetchOptions::default()).await?;
    assert_capture(&first, requested, final_url)?;
    check!(!first.from_cache);
    let replay = fetcher.get(requested, &FetchOptions::default()).await?;
    assert_capture(&replay, requested, final_url)?;
    check!(replay.from_cache);
    check!(eq; replay.fetched_at, first.fetched_at);
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", requested, ""));
    let (meta, body) = read_cache(
        &body_path,
        &meta_path,
        "GET",
        requested,
        &crate::net::RepresentationHeaders::default(),
    )?
    .ok_or("redirect cache missing")?;
    check!(eq; body, BODY);
    assert_archive(cache, &meta)?;
    let metadata_before = std::fs::read(&meta_path)?;
    let refreshed = fetcher
        .get(
            requested,
            &FetchOptions {
                refresh: true,
                headers: vec![("If-None-Match".to_string(), "\"capture-v1\"".to_string())],
                ..FetchOptions::default()
            },
        )
        .await?;
    assert_capture(&refreshed, requested, final_url)?;
    check!(eq; refreshed.fetched_at, first.fetched_at);
    check!(eq; std::fs::read(&meta_path)?, metadata_before);
    assert_archive(cache, &meta)?;
    Ok(first.fetched_at)
}

#[test]
fn actual_redirect_keeps_request_and_final_urls_through_archive_cache_offline_and_304() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let cache = root.path().join("http");
            let fetcher = Fetcher::new(
                &cache,
                None,
                Duration::ZERO,
                HashMap::new(),
                vec!["127.0.0.1".to_string()],
            )?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let origin = format!("http://{}", listener.local_addr()?);
            let requested = format!("{origin}/start");
            let final_url = format!("{origin}/finish");
            let ((), acquired_at) = tokio::time::timeout(Duration::from_secs(30), async {
                tokio::try_join!(
                    serve(listener),
                    acquire(&fetcher, &cache, &requested, &final_url)
                )
            })
            .await??;
            let offline = fetcher.with_offline(true);
            let replay = offline.get(&requested, &FetchOptions::default()).await?;
            assert_capture(&replay, &requested, &final_url)?;
            check!(replay.from_cache);
            check!(eq; replay.fetched_at, acquired_at);
            Ok(())
        })
}

#[test]
fn an_oversized_retry_after_is_refused_instead_of_parking_or_panicking() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let cache = root.path().join("http");
            let fetcher = Fetcher::new(
                &cache,
                None,
                Duration::ZERO,
                HashMap::new(),
                vec!["127.0.0.1".to_string()],
            )?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let origin = format!("http://{}", listener.local_addr()?);
            let requested = format!("{origin}/limited");
            let acquire = async {
                let outcome = fetcher.get(&requested, &FetchOptions::default()).await;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
            };
            let ((), outcome) = tokio::time::timeout(Duration::from_secs(30), async {
                tokio::try_join!(serve_rate_limited(listener), acquire)
            })
            .await??;
            check!(
                matches!(outcome, Err(FetchError::Http { status: 429, .. })),
                "the 429 keeps its status: {outcome:?}"
            );
            let conditions = fetcher.access_conditions().await;
            let refused = conditions
                .first()
                .ok_or("the 429 must record an access condition")?;
            check!(eq; refused.kind, AccessBlockKind::RateLimited);
            check!(eq;
                refused.retry_after_seconds, None,
                "an oversized Retry-After is refused, not recorded as the delay"
            );
            check!(
                refused.is_blocking(&crate::net::now_iso8601()),
                "the refusal still records the short policy cooldown"
            );
            check!(
                !refused.is_blocking("2100-01-01T00:00:00Z"),
                "the refused value must not park the host beyond the policy cooldown: {:?}",
                refused.cooldown_until
            );
            Ok(())
        })
}

#[test]
fn retry_after_a_future_http_date_is_honoured_end_to_end() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let cache = root.path().join("http");
            let fetcher = Fetcher::new(
                &cache,
                None,
                Duration::ZERO,
                HashMap::new(),
                vec!["127.0.0.1".to_string()],
            )?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let origin = format!("http://{}", listener.local_addr()?);
            let requested = format!("{origin}/limited");
            let deadline = chrono::Utc::now()
                .checked_add_signed(
                    chrono::TimeDelta::try_minutes(10).ok_or("a representable minute delta")?,
                )
                .ok_or("a representable HTTP-date instant")?;
            let retry_after = deadline.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
            let acquire = async {
                let outcome = fetcher.get(&requested, &FetchOptions::default()).await;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
            };
            let ((), outcome) = tokio::time::timeout(Duration::from_secs(30), async {
                tokio::try_join!(serve_date_limited(listener, retry_after), acquire)
            })
            .await??;
            check!(
                matches!(outcome, Err(FetchError::Http { status: 429, .. })),
                "the 429 keeps its status: {outcome:?}"
            );
            let conditions = fetcher.access_conditions().await;
            let limited = conditions
                .first()
                .ok_or("the 429 must record an access condition")?;
            check!(eq; limited.kind, AccessBlockKind::RateLimited);
            let seconds = limited
                .retry_after_seconds
                .ok_or("a future HTTP-date must not be dropped to the missing-header default")?;
            check!(
                seconds > crate::net::SHORT_COOLDOWN_SECONDS,
                "a future HTTP-date is its own deadline, never the 60s policy default: {seconds}"
            );
            check!(
                (590..=600).contains(&seconds),
                "the deadline is the server-named instant ten minutes out: {seconds}"
            );
            let cooldown = limited
                .cooldown_until
                .as_deref()
                .ok_or("a 429 with a Retry-After records a cooldown")?;
            let remaining = chrono::DateTime::parse_from_rfc3339(cooldown)?
                .signed_duration_since(chrono::Utc::now())
                .num_seconds();
            check!(
                (589..=601).contains(&remaining),
                "the recorded cooldown is the server-named deadline, not the policy default: {cooldown}"
            );
            check!(
                limited.is_blocking(&crate::net::now_iso8601()),
                "the host stays blocked for the server-named window"
            );
            Ok(())
        })
}
