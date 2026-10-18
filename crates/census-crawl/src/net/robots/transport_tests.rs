use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn serve(listener: TcpListener, responses: Vec<(&str, String)>) -> TestResult {
    for (expected, response) in responses {
        let (mut socket, _) = listener.accept().await?;
        let mut request = Vec::new();
        for _ in 0..32 {
            let mut buffer = [0_u8; 512];
            let count = socket.read(&mut buffer).await?;
            request.extend_from_slice(buffer.get(..count).ok_or("read boundary")?);
            if request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&request)?;
        check!(eq; request.split_whitespace().nth(1), Some(expected));
        socket.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

fn response(status: u16, body: &str) -> String {
    format!("HTTP/1.1 {status} Fixture\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn fetcher(path: &std::path::Path) -> Result<Fetcher, FetchError> {
    Fetcher::new(
        path,
        Some("census-service/0.1".to_owned()),
        Duration::ZERO,
        HashMap::new(),
        vec!["127.0.0.1".to_owned()],
    )
}

#[tokio::test]
async fn denied_redirect_is_refused_before_dispatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/start", listener.local_addr()?);
    let responses = vec![("/robots.txt", response(200, "User-agent: *\nDisallow: /private\n")),
        ("/start", "HTTP/1.1 302 Found\r\nLocation: /private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned())];
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, responses),
            fetcher.get_robots_checked(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("robots disallows"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 2);
    Ok(())
}

#[tokio::test]
async fn forbidden_robots_never_dispatches_school_page() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/school", listener.local_addr()?);
    let responses = vec![("/robots.txt", response(403, "forbidden"))];
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, responses),
            fetcher.get_robots_checked(&url, &options)
        )
    })
    .await?;
    served?;
    check!(matches!(result, Err(FetchError::Http { status: 403, .. })));
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn missing_robots_allows_page_and_cache_reuse_retains_provenance() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/school", listener.local_addr()?);
    let responses = vec![
        ("/robots.txt", response(404, "missing")),
        ("/school", response(200, "public")),
        ("/robots.txt", response(404, "missing")),
    ];
    let options = FetchOptions::default();
    let acquire = async {
        let capture = fetcher.get_robots_checked(&url, &options).await?;
        let replay = fetcher.get_robots_checked(&url, &options).await?;
        Ok::<_, FetchError>((capture, replay))
    };
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(serve(listener, responses), acquire)
    })
    .await?;
    served?;
    let (capture, replay) = result?;
    check!(replay.from_cache);
    check!(eq; replay.fetched_at, capture.fetched_at);
    check!(eq; replay.content_digest, capture.content_digest);
    check!(eq; fetcher.stats().await.physical_requests(), 3);
    Ok(())
}

#[tokio::test]
async fn shared_pacing_only_raises_delay_and_provider_refusal_stays_first() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    fetcher
        .raise_robots_delay("127.0.0.1", Duration::from_secs(3))
        .await?;
    fetcher
        .raise_robots_delay("127.0.0.1", Duration::ZERO)
        .await?;
    {
        let buckets = fetcher.pacing.hosts.lock().await;
        let state = buckets.get("127.0.0.1").ok_or("missing pacing state")?;
        check!(eq; state.delay, Duration::from_secs(3));
    }
    check!(matches!(
        fetcher
            .get_robots_checked("https://tfrrs.org/results", &FetchOptions::default())
            .await,
        Err(FetchError::Policy { .. })
    ));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn denied_initial_page_only_fetches_policy() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/private", listener.local_addr()?);
    let responses = vec![(
        "/robots.txt",
        response(200, "User-agent: *\nDisallow: /private\n"),
    )];
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, responses),
            fetcher.get_robots_checked(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("robots disallows"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn cached_final_destination_and_stale_policy_cannot_bypass_admission() -> TestResult {
    use crate::net::cache::{content_digest, write_cache, CacheMeta};
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?.with_offline(true);
    let root = "http://127.0.0.1";
    let requested = format!("{root}/start");
    let robots = format!("{root}/robots.txt");
    for (url, final_url, body) in [
        (
            robots.as_str(),
            robots.as_str(),
            b"User-agent: *\nDisallow: /private\n".as_slice(),
        ),
        (
            requested.as_str(),
            "http://127.0.0.1/private",
            b"historical page".as_slice(),
        ),
    ] {
        let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
        let meta = CacheMeta {
            url: url.to_owned(),
            response_url: Some(final_url.to_owned()),
            method: "GET".to_owned(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: crate::net::now_iso8601(),
            ..CacheMeta::default()
        };
        write_cache(&body_path, &meta_path, body, &meta)?;
    }
    check!(
        matches!(fetcher.get_robots_checked(&requested, &FetchOptions::default()).await,
        Err(FetchError::Policy { detail }) if detail.contains("robots disallows"))
    );
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &robots, ""));
    let meta = CacheMeta {
        url: robots,
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(b""),
        bytes: 0,
        fetched_at: "2000-01-01T00:00:00Z".to_owned(),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, b"", &meta)?;
    check!(
        matches!(fetcher.get_robots_checked(&requested, &FetchOptions::default()).await,
        Err(FetchError::Policy { detail }) if detail.contains("older"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn refreshed_304_cannot_replay_a_now_disallowed_effective_url() -> TestResult {
    use crate::net::cache::{content_digest, write_cache, CacheMeta};
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let url = format!("{origin}/start");
    let bytes = b"previously allowed private page";
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
    let meta = CacheMeta {
        url: url.clone(),
        response_url: Some(format!("{origin}/private")),
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(bytes),
        bytes: bytes.len(),
        fetched_at: crate::net::now_iso8601(),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, bytes, &meta)?;
    let responses = vec![
        (
            "/robots.txt",
            response(200, "User-agent: *\nDisallow: /private\n"),
        ),
        (
            "/start",
            "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_owned(),
        ),
    ];
    let options = FetchOptions {
        refresh: true,
        headers: vec![("If-None-Match".to_owned(), "\"previous-page\"".to_owned())],
        ..FetchOptions::default()
    };
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, responses),
            fetcher.get_robots_checked(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("robots disallows") && detail.contains("/private"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 2);
    check!(eq; std::fs::read(&body_path)?, bytes);
    Ok(())
}
