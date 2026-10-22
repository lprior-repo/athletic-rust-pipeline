use super::{parse_url, TransportScope};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

mod guards;
mod refusals;

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

#[test]
fn public_http_scope_preserves_scheme_host_port_and_credential_boundaries() -> TestResult {
    let origin = parse_url("https://school.example/")?.origin();
    let scope = TransportScope::PublicHttp(&origin);
    for target in [
        "http://school.example/",
        "https://school.example:8443/",
        "https://foreign.example/",
        "https://user:secret@school.example/",
    ] {
        check!(matches!(
            scope.check(target),
            Err(FetchError::Policy { .. })
        ));
    }
    scope.check("https://school.example/%73taff?view=all#faculty")?;
    Ok(())
}

#[tokio::test]
async fn public_page_never_requests_robots_and_reuse_retains_original_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/school", listener.local_addr()?);
    let options = FetchOptions::default();
    let acquire = async {
        let capture = fetcher.get_public_http(&url, &options).await?;
        let replay = fetcher.get_public_http(&url, &options).await?;
        Ok::<_, FetchError>((capture, replay))
    };
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, vec![("/school", response(200, "public"))]),
            acquire
        )
    })
    .await?;
    served?;
    let (capture, replay) = result?;
    check!(replay.from_cache);
    check!(eq; replay.fetched_at, capture.fetched_at);
    check!(eq; replay.content_digest, capture.content_digest);
    check!(eq; replay.body, capture.body);
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn historical_robots_cache_is_neither_required_read_as_policy_nor_rewritten() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?.with_offline(true);
    let url = "http://127.0.0.1/school";
    let robots = "http://127.0.0.1/robots.txt";
    cache(&fetcher, url, url, b"public")?;
    let (body, meta) = cache(
        &fetcher,
        robots,
        robots,
        b"User-agent: *\nDisallow: /\nCrawl-delay: NaN\n\xff",
    )?;
    let original = (std::fs::read(&body)?, std::fs::read(&meta)?);
    let captured = fetcher
        .get_public_http(url, &FetchOptions::default())
        .await?;
    check!(eq; captured.body, b"public");
    check!(eq; (std::fs::read(&body)?, std::fs::read(&meta)?), original);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

fn cache(
    fetcher: &Fetcher,
    url: &str,
    final_url: &str,
    body: &[u8],
) -> Result<(std::path::PathBuf, std::path::PathBuf), Box<dyn std::error::Error + Send + Sync>> {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    let meta = CacheMeta {
        url: url.to_owned(),
        response_url: Some(final_url.to_owned()),
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2000-01-01T00:00:00Z".to_owned(),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok((body_path, meta_path))
}
