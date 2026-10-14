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
        Box::pin(async { Err(std::io::Error::other("policy fixture forbids DNS").into()) })
    }
}

fn fetcher(root: &std::path::Path) -> TestResult<(Fetcher, Arc<AtomicUsize>)> {
    let mut fetcher = Fetcher::new(
        root,
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["gobound.com".into(), "127.0.0.1".into()],
    )?;
    let calls = Arc::new(AtomicUsize::new(0));
    fetcher.client = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
        .dns_resolver(Arc::new(NoDns(Arc::clone(&calls))))
        .build()?;
    Ok((fetcher, calls))
}

fn assert_policy(result: Result<FetchOutcome, FetchError>) -> TestResult {
    let Err(error @ FetchError::Policy { .. }) = result else {
        return Err(format!("expected GoBound policy refusal, got {result:?}").into());
    };
    check!(!error.retryable());
    let FetchError::Policy { detail } = error else {
        return Err("policy error changed variant".into());
    };
    check!(eq;
        Some(detail.as_str()),
        crate::registry::acquisition_refusal_for_source("bound")
    );
    Ok(())
}

#[tokio::test]
async fn acquisition_hold_precedes_dns_and_charging_for_get_post_refresh_and_ua_override(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    for url in [
        "https://gobound.com/ia/schools",
        "http://WWW.GoBound.COM.:8080/sd/schools",
        "https://staff.api.gobound.com/staff",
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
            assert_policy(fetcher.get(url, &options).await)?;
            assert_policy(
                fetcher
                    .post_json(url, &serde_json::json!({}), &options)
                    .await,
            )?;
            assert_policy(fetcher.post_form(url, &[], &options).await)?;
        }
    }
    check!(eq; dns.load(Ordering::SeqCst), 0);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn acquisition_hold_preserves_eligible_cache_and_offline_miss_semantics() -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    let url = "https://www.gobound.com/retained";
    let body = b"held bytes";
    let meta = CacheMeta {
        url: url.into(),
        method: "GET".into(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-07T10:00:00Z".into(),
        ..CacheMeta::default()
    };
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    write_cache(&body_path, &meta_path, body, &meta)?;
    let original_meta = std::fs::read(&meta_path)?;
    let cached = fetcher.get(url, &FetchOptions::default()).await?;
    check!(cached.from_cache);
    check!(eq; cached.fetched_at, meta.fetched_at);
    check!(eq; cached.body.as_slice(), body);
    assert_policy(
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
    check!(offline.get(url, &FetchOptions::default()).await?.from_cache);
    check!(matches!(
        offline
            .get("https://www.gobound.com/missing", &FetchOptions::default())
            .await,
        Err(FetchError::Offline { .. })
    ));
    check!(matches!(
        offline
            .get(
                "https://user:secret@www.gobound.com/retained",
                &FetchOptions::default(),
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
async fn acquisition_hold_stops_an_explicitly_granted_redirect_before_destination_dispatch(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (fetcher, dns) = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}/start");
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: http://www.gobound.com:{}/staff\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
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
    assert_policy(result)?;
    let stats = fetcher.stats().await;
    check!(eq; stats.physical_requests(), 1);
    check!(eq; stats.per_host.get("127.0.0.1").ok_or("first hop")?.physical_requests(), 1);
    check!(!stats.per_host.contains_key("www.gobound.com"));
    check!(eq; dns.load(Ordering::SeqCst), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}
