use crate::net::cache::{content_digest, read_cache, write_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type TestResult<T = (), E = Box<dyn std::error::Error + Send + Sync>> = Result<T, E>;

const BODY: &[u8] = b"original public capture";
const CAPTURED: &str = "2026-09-27T00:00:00Z";

#[derive(Clone, Copy)]
enum Mutation {
    None,
    SparseOversized,
    SameSizeCorruption,
}

async fn read_headers(socket: &mut TcpStream) -> TestResult<String> {
    let mut headers = Vec::new();
    let mut buffer = [0_u8; 512];
    for _ in 0..32 {
        let count = socket.read(&mut buffer).await?;
        if count == 0 {
            return Err("request ended before headers".into());
        }
        headers.extend_from_slice(&buffer[..count]);
        if headers.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            return Ok(String::from_utf8(headers)?);
        }
    }
    Err("request headers exceed fixture bound".into())
}

fn mutate(path: &Path, mutation: Mutation) -> TestResult {
    match mutation {
        Mutation::None => {}
        Mutation::SparseOversized => {
            std::fs::remove_file(path)?;
            std::fs::File::create(path)?.set_len(1_u64 << 40)?;
        }
        Mutation::SameSizeCorruption => {
            std::fs::remove_file(path)?;
            std::fs::write(path, vec![b'x'; BODY.len()])?;
        }
    }
    Ok(())
}

async fn serve(listener: TcpListener, body_path: PathBuf, mutation: Mutation) -> TestResult {
    for _ in 0..2 {
        let (mut socket, _) = listener.accept().await?;
        let request = read_headers(&mut socket).await?;
        let response = if request.starts_with("GET /robots.txt ") {
            let robots = "User-agent: *\r\nAllow: /\r\n";
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{robots}",
                robots.len()
            )
        } else {
            check!(request.starts_with("GET /payload "), "{request}");
            check!(
                request
                    .to_ascii_lowercase()
                    .contains("if-none-match: \"capture-v1\""),
                "{request}"
            );
            mutate(&body_path, mutation)?;
            "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string()
        };
        socket.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

fn original_meta(url: &str) -> CacheMeta {
    CacheMeta {
        url: url.to_string(),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: content_digest(BODY),
        bytes: BODY.len(),
        fetched_at: CAPTURED.to_string(),
        etag: Some("\"capture-v1\"".to_string()),
        last_modified: None,
        content_type: Some("text/plain".to_string()),
    }
}

fn verify_archive(cache: &Path, meta: &CacheMeta) -> TestResult {
    let body = cache
        .join("archive/bodies")
        .join(format!("{}.body", meta.content_digest));
    check!(eq; std::fs::read(&body)?, BODY);
    let captures = cache.join("archive/captures").join(&meta.content_digest);
    let mut found = false;
    for entry in std::fs::read_dir(captures)? {
        let entry = entry?;
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(entry.path())?)?;
        check!(eq; value["fetched_at"], CAPTURED);
        check!(eq; value["status"], 200);
        check!(eq; value["content_digest"], meta.content_digest);
        check!(eq; value["bytes"], BODY.len());
        found = true;
    }
    check!(found, "original capture metadata must remain archived");
    Ok(())
}

async fn conditional(mutation: Mutation) -> TestResult {
    let directory = tempfile::tempdir()?;
    let cache = directory.path().join("http");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        vec!["127.0.0.1".to_string()],
    )?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/payload", listener.local_addr()?);
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
    let meta = original_meta(&url);
    write_cache(&body_path, &meta_path, BODY, &meta)?;
    let metadata_before = std::fs::read(&meta_path)?;
    let response = async {
        let outcome = fetcher
            .get(
                &url,
                &FetchOptions {
                    refresh: true,
                    headers: vec![("If-None-Match".to_string(), "\"capture-v1\"".to_string())],
                    ..FetchOptions::default()
                },
            )
            .await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
    };
    let ((), outcome) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(serve(listener, body_path.clone(), mutation), response)
    })
    .await??;
    match mutation {
        Mutation::None => {
            let outcome = outcome?;
            check!(eq; outcome.body, BODY);
            check!(eq; outcome.fetched_at, CAPTURED);
            check!(eq; outcome.response_url, None);
            check!(eq; outcome.content_digest, meta.content_digest);
            check!(eq; outcome.status, 200);
            check!(!outcome.from_cache);
            let (cached, body) =
                read_cache(&body_path, &meta_path)?.ok_or("valid replay lost cache")?;
            check!(eq; cached.fetched_at, CAPTURED);
            check!(eq; cached.response_url, None);
            check!(eq; body, BODY);
            check!(eq; fetcher.stats().await.conditional_304, 1);
        }
        Mutation::SparseOversized | Mutation::SameSizeCorruption => {
            check!(
                matches!(outcome, Err(FetchError::Http { status: 304, .. })),
                "{outcome:?}"
            );
            check!(eq; std::fs::read(&meta_path)?, metadata_before);
            check!(eq; fetcher.stats().await.conditional_304, 0);
        }
    }
    verify_archive(&cache, &meta)?;
    Ok(())
}

#[test]
fn actual_304_retains_original_acquisition_in_cache_outcome_and_archive() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { conditional(Mutation::None).await })
}

#[test]
fn cache_replaced_by_a_terabyte_sparse_file_during_conditional_dispatch_is_bounded_rejection(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { conditional(Mutation::SparseOversized).await })
}

#[test]
fn same_size_cache_corruption_during_conditional_dispatch_cannot_become_a_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { conditional(Mutation::SameSizeCorruption).await })
}
