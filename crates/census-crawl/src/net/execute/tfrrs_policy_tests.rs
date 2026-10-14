use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct NoDns(Arc<AtomicUsize>);

impl reqwest::dns::Resolve for NoDns {
    fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(std::io::Error::other("TFRRS fixture forbids DNS").into()) })
    }
}

fn fetcher(root: &std::path::Path) -> TestResult<(Fetcher, Arc<AtomicUsize>)> {
    let mut fetcher = Fetcher::new(
        root,
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["tfrrs.org".into(), "127.0.0.1".into()],
    )?;
    let calls = Arc::new(AtomicUsize::new(0));
    fetcher.client = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
        .dns_resolver(Arc::new(NoDns(Arc::clone(&calls))))
        .build()?;
    Ok((fetcher, calls))
}

fn assert_terms(result: Result<FetchOutcome, FetchError>) -> TestResult {
    let Err(error @ FetchError::Policy { .. }) = result else {
        return Err(format!("expected TFRRS terms refusal, got {result:?}").into());
    };
    check!(!error.retryable());
    let FetchError::Policy { detail } = error else {
        return Err("policy error changed variant".into());
    };
    check!(eq;
        Some(detail.as_str()),
        crate::registry::acquisition_refusal_for_source("tfrrs")
    );
    Ok(())
}

#[tokio::test]
async fn tfrrs_terms_hold_precedes_dns_and_charging_across_methods_refresh_and_headers(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    for url in [
        "https://tfrrs.org/results",
        "http://INDIANA.TFRRS.ORG.:8080/lists",
        "https://results.nh.tfrrs.org/lists",
    ] {
        for refresh in [false, true] {
            let options = FetchOptions {
                refresh,
                headers: vec![(
                    "user-agent".into(),
                    "Mozilla/5.0 census-policy-fixture".into(),
                )],
                ..FetchOptions::default()
            };
            assert_terms(fetcher.get(url, &options).await)?;
            assert_terms(
                fetcher
                    .post_json(url, &serde_json::json!({}), &options)
                    .await,
            )?;
            assert_terms(fetcher.post_form(url, &[], &options).await)?;
        }
    }
    check!(eq; dns.load(Ordering::SeqCst), 0);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn tfrrs_terms_hold_preserves_eligible_retained_bytes_and_offline_security_checks(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    let url = "https://indiana.tfrrs.org/retained";
    let body = b"historical TFRRS bytes";
    let meta = CacheMeta {
        url: url.into(),
        method: "GET".into(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-22T03:55:31Z".into(),
        ..CacheMeta::default()
    };
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    write_cache(&body_path, &meta_path, body, &meta)?;
    let original_meta = std::fs::read(&meta_path)?;
    let retained = fetcher.get(url, &FetchOptions::default()).await?;
    check!(retained.from_cache);
    check!(eq; retained.fetched_at, meta.fetched_at);
    check!(eq; retained.body.as_slice(), body);
    assert_terms(
        fetcher
            .get(
                url,
                &FetchOptions {
                    refresh: true,
                    ..FetchOptions::default()
                },
            )
            .await,
    )?;
    let offline = fetcher.with_offline(true);
    let replayed = offline.get(url, &FetchOptions::default()).await?;
    check!(replayed.from_cache);
    check!(eq; replayed.fetched_at, meta.fetched_at);
    check!(eq; replayed.body.as_slice(), body);
    check!(matches!(
        offline
            .get(
                "https://indiana.tfrrs.org/missing",
                &FetchOptions::default()
            )
            .await,
        Err(FetchError::Offline { .. })
    ));
    check!(matches!(
        offline
            .get(
                "https://user:secret@indiana.tfrrs.org/retained",
                &FetchOptions::default()
            )
            .await,
        Err(FetchError::Policy { .. })
    ));
    check!(eq; std::fs::read(&meta_path)?, original_meta);
    check!(eq; std::fs::read(&body_path)?.as_slice(), body);
    check!(eq; dns.load(Ordering::SeqCst), 0);
    check!(eq; offline.stats().await.physical_requests(), 0);
    check!(offline.access_conditions().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn tfrrs_terms_hold_refuses_an_explicitly_granted_redirect_before_destination_dispatch(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}/start");
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: http://indiana.tfrrs.org:{}/lists\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        address.port()
    );
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let mut request = [0_u8; 4096];
        let read = socket.read(&mut request).await?;
        check!(read > 0);
        socket.write_all(response.as_bytes()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(server, fetcher.get(&url, &options))
    })
    .await?;
    served?;
    assert_terms(result)?;
    let stats = fetcher.stats().await;
    check!(eq; stats.physical_requests(), 1);
    check!(eq; stats.per_host.get("127.0.0.1").ok_or("first hop")?.physical_requests(), 1);
    check!(!stats.per_host.contains_key("indiana.tfrrs.org"));
    check!(eq; dns.load(Ordering::SeqCst), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}
