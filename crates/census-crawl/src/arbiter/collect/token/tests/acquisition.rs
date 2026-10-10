use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::FetchError;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod foreign;

struct Reply {
    path: String,
    status: u16,
    location: Option<String>,
    body: Vec<u8>,
}

fn reply(path: &str, status: u16, body: &str) -> Reply {
    Reply {
        path: path.into(),
        status,
        location: None,
        body: body.as_bytes().to_vec(),
    }
}

fn redirect() -> Reply {
    Reply {
        location: Some("/directory/shell/index.html".into()),
        ..reply("/directory/", 302, "")
    }
}

async fn serve(listener: tokio::net::TcpListener, replies: Vec<Reply>) -> TestResult {
    for response in replies {
        let (mut socket, _) = listener.accept().await?;
        let mut request = Vec::new();
        for _ in 0..16 {
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await?;
            request.extend_from_slice(&bytes[..count]);
            if count == 0 || request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&request)?;
        check!(eq; request.lines().next(), Some(format!("GET {} HTTP/1.1", response.path).as_str()));
        let location = response
            .location
            .map_or_else(String::new, |url| format!("Location: {url}\r\n"));
        let header = format!(
            "HTTP/1.1 {} Fixture\r\n{location}Content-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
            response.status, response.body.len()
        );
        socket.write_all(header.as_bytes()).await?;
        socket.write_all(&response.body).await?;
    }
    Ok(())
}

fn fetcher(cache: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        vec!["127.0.0.1".into()],
    )?)
}

fn archived(
    cache: &Path,
    requested: &str,
    observed: Option<&str>,
    body: &[u8],
) -> TestResult<serde_json::Value> {
    let digest = content_digest(body);
    let meta: serde_json::Value = serde_json::from_slice(&std::fs::read(cache.join(format!(
        "{}.meta.json",
        Fetcher::key_for("GET", requested, "")
    )))?)?;
    check!(eq; meta["url"], requested);
    check!(eq; meta.get("response_url").and_then(serde_json::Value::as_str), observed);
    check!(eq; meta["method"], "GET");
    check!(eq; meta["status"], 200);
    check!(eq; meta["content_digest"], digest);
    check!(eq; meta["bytes"], body.len());
    check!(eq; std::fs::read(cache.join("archive/bodies").join(format!("{digest}.body")))?, body);
    let captures: Vec<serde_json::Value> =
        std::fs::read_dir(cache.join("archive/captures").join(digest))?
            .map(|entry| -> TestResult<serde_json::Value> {
                Ok(serde_json::from_slice(&std::fs::read(entry?.path())?)?)
            })
            .collect::<TestResult<_>>()?;
    check!(captures.contains(&meta));
    Ok(meta)
}

#[test]
fn an_owned_loopback_entry_redirect_and_redeployment_acquire_only_the_declared_bundle() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let host = format!("http://{}", listener.local_addr()?);
    let entry_url = format!("{host}/directory/");
    let observed = format!("{host}/directory/shell/index.html");
    let first = "<script type=module src='../assets/index-loopback-old.js'></script>";
    let second = "<template><script type=module src='https://foreign.invalid/no.js'></script></template><script type=module src='../assets/index-loopback-new.js'></script>";
    let first_bundle = "export const fixtureRevision = 1;";
    let next_bundle = "export const fixtureRevision = 2;";
    let replies = vec![
        redirect(),
        reply("/directory/shell/index.html", 200, first),
        reply("/directory/assets/index-loopback-old.js", 200, first_bundle),
        redirect(),
        reply("/directory/shell/index.html", 200, second),
        reply("/directory/assets/index-loopback-new.js", 200, next_bundle),
    ];
    let client = async {
        let fetcher = fetcher(&cache)?;
        let fetch = FetchOptions { refresh: true, ..FetchOptions::default() };
        let prior = acquire_bundle(&fetcher, &entry_url, &fetch).await?;
        let prior_url = format!("{host}/directory/assets/index-loopback-old.js");
        check!(eq; prior.url, prior_url);
        check!(eq; prior.response_url.as_deref(), Some(prior_url.as_str()));
        check!(eq; prior.body, first_bundle.as_bytes());
        let old_entry_meta = archived(&cache, &entry_url, Some(&observed), first.as_bytes())?;
        let old_bundle_meta = archived(&cache, &prior_url, Some(&prior_url), first_bundle.as_bytes())?;
        let next = acquire_bundle(&fetcher, &entry_url, &fetch).await?;
        let next_url = format!("{host}/directory/assets/index-loopback-new.js");
        check!(eq; next.url, next_url);
        check!(eq; next.response_url.as_deref(), Some(next_url.as_str()));
        check!(eq; next.body, next_bundle.as_bytes());
        archived(&cache, &entry_url, Some(&observed), second.as_bytes())?;
        archived(&cache, &next_url, Some(&next_url), next_bundle.as_bytes())?;
        let old_digest = content_digest(first.as_bytes());
        let retained: Vec<serde_json::Value> = std::fs::read_dir(cache.join("archive/captures").join(old_digest))?
            .map(|entry| -> TestResult<serde_json::Value> {
                Ok(serde_json::from_slice(&std::fs::read(entry?.path())?)?)
            })
            .collect::<TestResult<_>>()?;
        check!(retained.iter().any(|meta| meta == &old_entry_meta));
        archived(&cache, &prior_url, Some(&prior_url), first_bundle.as_bytes())?;
        check!(eq; old_bundle_meta["url"], prior_url);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (served, acquired) = tokio::join!(serve(listener, replies), client);
        served?;
        acquired
    }).await?
    })
}

#[test]
fn historical_cached_entry_and_bundle_keep_the_observed_final_url_unknown() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            let fetcher = fetcher(&cache)?.with_offline(true);
            let bundle_url = "https://live.arbiter.io/directory/assets/index-history.js";
            let html = b"<script type=module src=assets/index-history.js></script>";
            let body = b"export const historicalFixture = true;";
            for (url, bytes) in [
                (DIRECTORY_URL, html.as_slice()),
                (bundle_url, body.as_slice()),
            ] {
                let key = Fetcher::key_for("GET", url, "");
                write_cache(
                    &cache.join(format!("{key}.body")),
                    &cache.join(format!("{key}.meta.json")),
                    bytes,
                    &CacheMeta {
                        redirects: Vec::new(),
                        representation: crate::net::RepresentationHeaders::default(),
                        url: url.to_string(),
                        method: "GET".into(),
                        status: 200,
                        content_digest: content_digest(bytes),
                        bytes: bytes.len(),
                        fetched_at: "2026-10-02T12:00:00Z".into(),
                        ..CacheMeta::default()
                    },
                )?;
            }
            let bundle = acquire_bundle(&fetcher, DIRECTORY_URL, &FetchOptions::default()).await?;
            check!(eq; bundle.url, bundle_url);
            check!(eq; bundle.response_url, None);
            check!(eq; bundle.body, body);
            archived(&cache, DIRECTORY_URL, None, html)?;
            archived(&cache, bundle_url, None, body)?;
            Ok(())
        })
}

#[test]
fn entry_and_module_http_failures_preserve_their_typed_retry_disposition() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    for module_failure in [false, true] {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let host = format!("http://{}", listener.local_addr()?);
        let entry_url = format!("{host}/directory/");
        let mut replies = vec![];
        if module_failure {
            replies.push(reply("/directory/", 200, "<script type=module src=assets/index-failed.js></script>"));
            replies.push(reply("/directory/assets/index-failed.js", 503, "fixture unavailable"));
        } else {
            replies.push(reply("/directory/", 503, "fixture unavailable"));
        }
        let client = async {
            let fetcher = fetcher(&cache)?;
            let expected = if module_failure { format!("{host}/directory/assets/index-failed.js") } else { entry_url.clone() };
            match acquire_bundle(&fetcher, &entry_url, &FetchOptions::default()).await {
                Err(CrawlError::Fetch(failure)) => {
                    check!(matches!(&failure, FetchError::Http { status: 503, url } if url == &expected));
                    check!(failure.retryable());
                }
                Err(error) => return Err(error.into()),
                Ok(_) => return Err("source failure".into()),
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(Duration::from_secs(15), async {
            let (served, acquired) = tokio::join!(serve(listener, replies), client);
            served?;
            acquired
        }).await??;
    }
    Ok(())
    })
}

#[test]
fn unsafe_or_ambiguous_entry_html_refuses_before_any_bundle_acquisition() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    for html in [
        "<script type=module src='https://foreign.invalid/directory/assets/no.js'></script>",
        "<script type=module src=assets/one.js></script><script type=module src=assets/two.js></script>",
        "<script type=module src=assets/one.js></script><script type=module src='assets/two.js'",
    ] {
        let dir = tempfile::tempdir()?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let entry_url = format!("http://{}/directory/", listener.local_addr()?);
        let replies = vec![
            reply("/directory/", 200, html),
        ];
        let client = async {
            let fetcher = fetcher(&dir.path().join("http"))?;
            match acquire_bundle(&fetcher, &entry_url, &FetchOptions::default()).await {
                Err(CrawlError::Schema { url, .. }) => check!(eq; url, entry_url),
                Err(error) => return Err(error.into()),
                Ok(_) => return Err("entry fails closed".into()),
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(Duration::from_secs(15), async {
            let (served, acquired) = tokio::join!(serve(listener, replies), client);
            served?;
            acquired
        }).await??;
    }
    Ok(())
    })
}

#[test]
fn a_deleted_discovered_asset_remains_a_typed_terminal_404_without_a_fallback() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let host = format!("http://{}", listener.local_addr()?);
    let entry_url = format!("{host}/directory/");
    let module_url = format!("{host}/directory/assets/index-deleted.js");
    let replies = vec![
        reply("/directory/", 200, "<script type=module src=assets/index-deleted.js></script>"),
        reply("/directory/assets/index-deleted.js", 404, "fixture deleted"),
    ];
    let client = async {
        let fetcher = fetcher(&dir.path().join("http"))?;
        match acquire_bundle(&fetcher, &entry_url, &FetchOptions::default()).await {
            Err(CrawlError::Fetch(failure)) => {
                check!(matches!(&failure, FetchError::Http { status: 404, url } if url == &module_url));
                check!(!failure.retryable());
            }
            Err(error) => return Err(error.into()),
            Ok(_) => return Err("deleted declared asset".into()),
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (served, acquired) = tokio::join!(serve(listener, replies), client);
        served?;
        acquired
    }).await?
    })
}
