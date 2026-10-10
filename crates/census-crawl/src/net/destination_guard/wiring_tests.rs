use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SILENCE_MS: u64 = 50;

fn fetcher_with(authorized: Vec<String>) -> TestResult<(Fetcher, tempfile::TempDir)> {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        authorized,
    )?;
    Ok((fetcher, dir))
}

async fn read_path(socket: &mut tokio::net::TcpStream) -> TestResult<String> {
    let mut bytes = [0; 4096];
    let length = socket.read(&mut bytes).await?;
    let request = std::str::from_utf8(bytes.get(..length).ok_or("request boundary")?)?;
    Ok(request
        .split_whitespace()
        .nth(1)
        .ok_or("request path")?
        .to_string())
}

fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn redirect(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

async fn assert_silent(listener: &tokio::net::TcpListener) -> TestResult {
    if tokio::time::timeout(Duration::from_millis(SILENCE_MS), listener.accept())
        .await
        .is_ok()
    {
        return Err("the listener was contacted".into());
    }
    Ok(())
}

async fn get(fetcher: &Fetcher, url: &str) -> Result<FetchOutcome, FetchError> {
    fetcher.get(url, &FetchOptions::default()).await
}

#[test]
fn an_unauthorized_loopback_literal_never_opens_a_connection() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let (fetcher, _dir) = fetcher_with(Vec::new())?;
            let url = format!("http://127.0.0.1:{}/payload", address.port());
            let error = get(&fetcher, &url).await.err().ok_or("fetched")?;
            check!(matches!(error, FetchError::Policy { .. }), "{error:?}");
            assert_silent(&listener).await
        })
}

#[test]
fn a_localhost_domain_without_a_grant_never_opens_a_connection() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let (fetcher, _dir) = fetcher_with(vec!["example.com".to_string()])?;
            let url = format!("http://localhost:{}/payload", address.port());
            let error = get(&fetcher, &url).await.err().ok_or("fetched")?;
            check!(matches!(error, FetchError::Policy { .. }), "{error:?}");
            assert_silent(&listener).await
        })
}

#[test]
fn an_explicit_loopback_grant_admits_the_fixture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let server = async move {
                let mut paths = Vec::new();
                let (mut socket, _) = listener.accept().await?;
                let path = read_path(&mut socket).await?;
                let response = match path.as_str() {
                    "/payload" => ok("evidence"),
                    other => {
                        return Err::<Vec<String>, _>(
                            format!("unapproved path contacted: {other}").into(),
                        )
                    }
                };
                paths.push(path);
                socket.write_all(response.as_bytes()).await?;
                assert_silent(&listener).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(paths)
            };
            let (fetcher, _dir) = fetcher_with(vec!["127.0.0.1".to_string()])?;
            let requests = async {
                let url = format!("http://127.0.0.1:{}/payload", address.port());
                let outcome = get(&fetcher, &url).await?;
                check!(eq; outcome.status, 200);
                check!(eq; outcome.text(), "evidence");
                check!(!outcome.from_cache);
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            };
            let (paths, ()) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::try_join!(server, requests)
            })
            .await??;
            check!(eq; paths, ["/payload"]);
            Ok(())
        })
}

#[test]
fn a_same_host_redirect_is_followed_and_a_local_hop_is_refused() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let unlisted = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let unlisted_address = unlisted.local_addr()?;
            let hop = format!("http://127.0.0.1:{}/hidden", unlisted_address.port());
            let server = async move {
                let mut paths = Vec::new();
                for _ in 0..3 {
                    let (mut socket, _) = listener.accept().await?;
                    let path = read_path(&mut socket).await?;
                    let response = match path.as_str() {
                        "/start" => redirect("/finish"),
                        "/finish" => ok("evidence"),
                        "/cross" => redirect(&hop),
                        other => {
                            return Err::<Vec<String>, _>(
                                format!("unapproved path contacted: {other}").into(),
                            )
                        }
                    };
                    paths.push(path);
                    socket.write_all(response.as_bytes()).await?;
                }
                assert_silent(&listener).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(paths)
            };
            let (fetcher, _dir) = fetcher_with(vec!["localhost".to_string()])?;
            let requests = async {
                let start = format!("http://localhost:{}/start", address.port());
                let outcome = get(&fetcher, &start).await?;
                check!(eq; outcome.text(), "evidence");
                let cross = format!("http://localhost:{}/cross", address.port());
                let error = get(&fetcher, &cross)
                    .await
                    .err()
                    .ok_or("unlisted hop followed")?;
                check!(
                    matches!(&error, FetchError::Policy { detail } if detail.contains("bypasses admission")),
                    "{error:?}"
                );
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            };
            let (paths, ()) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::try_join!(server, requests)
            })
            .await??;
            check!(eq; paths, ["/start", "/finish", "/cross"]);
            assert_silent(&unlisted).await
        })
}

#[test]
fn a_cached_body_does_not_bypass_an_unauthorized_destination() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fetcher, dir) = fetcher_with(Vec::new())?;
            let url = "http://127.0.0.1:9/payload";
            let cache = dir.path().join("http");
            std::fs::create_dir_all(&cache)?;
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(b"GET");
            hasher.update([0x1f]);
            hasher.update(url.as_bytes());
            hasher.update([0x1f]);
            let key: String = hasher.finalize()[..16]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            std::fs::write(cache.join(format!("{key}.body")), "seeded")?;
            std::fs::write(
                cache.join(format!("{key}.meta.json")),
                serde_json::json!({
                    "url": url,
                    "method": "GET",
                    "status": 200,
                    "content_digest": format!("{:x}", Sha256::digest(b"seeded")),
                    "bytes": 6,
                    "fetched_at": "2026-09-30T00:00:00Z",
                })
                .to_string(),
            )?;
            let error = get(&fetcher, url)
                .await
                .err()
                .ok_or("a cached refused url was served")?;
            check!(matches!(error, FetchError::Policy { .. }), "{error:?}");
            Ok(())
        })
}

#[test]
fn non_http_schemes_and_credential_bearing_urls_are_refused() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fetcher, _dir) = fetcher_with(vec!["example.com".to_string()])?;
            for url in [
                "file:///etc/passwd",
                "ftp://example.com/payload",
                "http://user:secret@example.com/payload",
            ] {
                let error = get(&fetcher, url).await.err().ok_or("fetched")?;
                check!(
                    matches!(error, FetchError::Policy { .. }),
                    "{url}: {error:?}"
                );
            }
            Ok(())
        })
}
