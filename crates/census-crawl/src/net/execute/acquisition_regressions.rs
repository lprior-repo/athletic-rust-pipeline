use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::time::Duration;
use tokio::net::TcpListener;

mod fixture;
use fixture::{fetcher, independent_representations, options, serve, TestResult};

#[test]
fn cache_replay_refuses_a_foreign_request_identity_and_separates_vary_accept() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let fetcher = fetcher(root.path())?.with_offline(true);
            let requested = "https://example.test/a";
            let (body_path, meta_path) =
                fetcher.cache_paths(&Fetcher::key_for("GET", requested, ""));
            let selected = crate::net::RepresentationHeaders::canonical(&[(
                "Accept".into(),
                "application/json".into(),
            )])?;
            for (url, method, representation) in [
                (
                    "https://example.test/b",
                    "GET",
                    crate::net::RepresentationHeaders::default(),
                ),
                (
                    requested,
                    "POST",
                    crate::net::RepresentationHeaders::default(),
                ),
                (requested, "GET", selected),
            ] {
                let meta = CacheMeta {
                    redirects: Vec::new(),
                    url: url.into(),
                    method: method.into(),
                    representation,
                    status: 200,
                    content_digest: content_digest(b"foreign"),
                    bytes: 7,
                    fetched_at: "2026-09-27T00:00:00Z".into(),
                    ..CacheMeta::default()
                };
                write_cache(&body_path, &meta_path, b"foreign", &meta)?;
                check!(
                    matches!(fetcher.get(requested, &FetchOptions::default()).await,
                Err(FetchError::Offline { url }) if url == requested)
                );
            }
            independent_representations(
                [
                    options(&[("Accept", "text/html"), ("Accept-Language", "en")]),
                    options(&[("Accept", "application/json"), ("Accept-Language", "en")]),
                    options(&[("accept-language", "en"), ("ACCEPT", "text/html")]),
                ],
                [b"public html", b"{\"value\":1}"],
            )
            .await
        })
}

#[test]
fn accept_language_selects_independently_replayable_captures() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            independent_representations(
                [
                    options(&[("Accept-Language", "en")]),
                    options(&[("Accept-Language", "fr")]),
                    options(&[("accept-language", "en")]),
                ],
                [b"public html", b"bonjour"],
            )
            .await
        })
}

#[test]
fn validators_share_cache_identity_and_preserve_conditional_dispatch() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let fetcher = fetcher(root.path())?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let url = format!("http://{}/payload", listener.local_addr()?);
            let client = async {
                let original = fetcher.get(&url, &FetchOptions::default()).await?;
                for header in [
                    ("If-None-Match", "\"v1\""),
                    ("If-Modified-Since", "Tue, 29 Sep 2026 12:00:00 GMT"),
                ] {
                    let mut request = options(&[header]);
                    let replay = fetcher.get(&url, &request).await?;
                    check!(replay.from_cache);
                    check!(eq; replay.body, original.body);
                    request.refresh = true;
                    let conditional = fetcher.get(&url, &request).await?;
                    check!(!conditional.from_cache);
                    check!(eq; conditional.body, original.body);
                    check!(eq; conditional.fetched_at, original.fetched_at);
                }
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            };
            let (requests, ()) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(serve(listener, 3), client)
            })
            .await??;
            check!(requests.iter().any(|request| request
                .to_ascii_lowercase()
                .contains("if-none-match: \"v1\"")));
            check!(requests.iter().any(|request| request
                .to_ascii_lowercase()
                .contains("if-modified-since: tue, 29 sep 2026 12:00:00 gmt")));
            check!(eq; fetcher.stats().await.conditional_304, 2);
            Ok(())
        })
}

#[test]
fn physical_request_accounting_agrees_across_aggregate_and_per_host() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let root = tempfile::tempdir()?;
        let fetcher = fetcher(root.path())?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("http://{}", listener.local_addr()?);
        let client = async {
            let url = format!("{origin}/redirect");
            let original = fetcher.get(&url, &FetchOptions::default()).await?;
            let replay = fetcher.get(&url, &FetchOptions::default()).await?;
            check!(replay.from_cache);
            check!(eq; replay.body, original.body);
            let mut conditional = options(&[("If-None-Match", "\"v1\"")]);
            conditional.refresh = true;
            check!(eq; fetcher.get(&format!("{origin}/payload"), &FetchOptions::default()).await?.body, original.body);
            check!(eq; fetcher.get(&format!("{origin}/payload"), &conditional).await?.body, original.body);
            check!(matches!(fetcher.get(&format!("{origin}/limited"), &FetchOptions::default()).await,
                Err(FetchError::Http { status: 429, .. })));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        };
        let (requests, ()) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::try_join!(serve(listener, 5), client)
        }).await??;
        let stats = fetcher.stats().await;
        let observed = u64::try_from(requests.len())?;
        check!(eq; observed, 5);
        check!(eq; stats.physical_requests(), observed);
        check!(eq; stats.per_host.values().map(|host| host.physical_requests()).sum::<u64>(), observed);
        check!(eq; stats.requests, 6);
        check!(eq; stats.cache_hits, 1);
        check!(eq; stats.conditional_304, 1);
        check!(eq; stats.useful_records_per_physical_request(12), Some(2.4));
        Ok(())
    })
}

#[test]
fn empty_representation_replays_historical_bytes_without_advancing_freshness() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let fetcher = fetcher(root.path())?.with_offline(true);
            let url = "https://example.test/historical";
            let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
            let body = b"unchanged historical acquisition";
            let metadata = serde_json::to_vec(&serde_json::json!({
                "url": url, "method": "GET", "status": 200, "content_digest": content_digest(body),
                "bytes": body.len(), "fetched_at": "2026-09-27T00:00:00Z",
            }))?;
            std::fs::write(&body_path, body)?;
            std::fs::write(&meta_path, &metadata)?;
            let outcome = fetcher.get(url, &FetchOptions::default()).await?;
            check!(outcome.from_cache);
            check!(eq; outcome.body, body);
            check!(eq; outcome.fetched_at, "2026-09-27T00:00:00Z");
            check!(eq; std::fs::read(meta_path)?, metadata);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}
