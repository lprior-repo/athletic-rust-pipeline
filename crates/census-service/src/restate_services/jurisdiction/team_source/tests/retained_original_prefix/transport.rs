use super::{codec, prefix, TestResult};
use crate::restate_services::jurisdiction::{JurisdictionCensus, TeamsSource};
use crate::restate_services::{CaptureExecution, Jobs, NationalCensus};
use census_store::{clock::Clock, Store};
use restate_sdk::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub(super) struct UnusedClock(pub(super) AtomicUsize);

impl Clock for UnusedClock {
    fn today(&self) -> String {
        self.0.fetch_add(1, Ordering::SeqCst);
        "2099-12-31".into()
    }

    fn today_iso8601(&self) -> String {
        self.0.fetch_add(1, Ordering::SeqCst);
        "2099-12-31T23:59:59Z".into()
    }

    fn now(&self) -> tokio::time::Instant {
        self.0.fetch_add(1, Ordering::SeqCst);
        tokio::time::Instant::now()
    }
}

pub(super) async fn exchange(
    store: &Arc<Store>,
    clock: &Arc<UnusedClock>,
    prefix: &prefix::Prefix,
) -> TestResult<Vec<codec::Frame>> {
    exchange_recorded(
        store,
        clock,
        CaptureExecution::LegacyFresh,
        &prefix::Journal {
            key: prefix::KEY,
            invocation: prefix::INVOCATION,
            frames: prefix.frames.clone(),
        },
        "TeamsSource/run",
    )
    .await
}

pub(super) async fn exchange_recorded(
    store: &Arc<Store>,
    clock: &Arc<UnusedClock>,
    capture: CaptureExecution,
    prefix: &prefix::Journal,
    handler: &str,
) -> TestResult<Vec<codec::Frame>> {
    let region = Arc::new(crate::spawn::Spawner::new());
    let owner = JurisdictionCensus::new(
        Arc::clone(store),
        clock.clone(),
        None,
        capture,
        Jobs::new(Arc::clone(store), Arc::clone(&region)),
    );
    let browser_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let browser_transport = reqwest::Client::builder().no_proxy().build()?;
    let lane = census_crawl::net::bridge::BrowserLane::over(census_crawl::ingress::client(
        format!("http://{}/", browser_listener.local_addr()?).parse()?,
        browser_transport,
    )?);
    let fetcher = Arc::new(
        census_crawl::net::Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_offline(true)
        .with_browser_lane(lane),
    );
    *owner.fetcher.lock().await = Some((Vec::new(), 4, Arc::clone(&fetcher)));
    let source = TeamsSource::new(
        owner.clone(),
        Jobs::new(Arc::clone(store), Arc::clone(&region)),
    );
    let national = NationalCensus::new(
        clock.clone(),
        Jobs::new(Arc::clone(store), Arc::clone(&region)),
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!("http://{}/invoke/{handler}", listener.local_addr()?);
    let endpoint = Endpoint::builder()
        .bind(source)
        .bind(owner)
        .bind(national)
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let result = tokio::time::timeout(Duration::from_secs(10), invoke(&url, prefix)).await;
        stop_tx
            .send(())
            .map_err(|_| "retained-prefix endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    result
}

async fn invoke(url: &str, prefix: &prefix::Journal) -> TestResult<Vec<codec::Frame>> {
    let mut start = codec::blob(1, b"retained-original-prefix")?;
    start.extend(codec::blob(2, prefix.invocation.as_bytes())?);
    start.extend(codec::integer(3, u64::try_from(prefix.frames.len())?));
    start.extend(codec::blob(6, prefix.key.as_bytes())?);
    let mut body = codec::Frame::new(0, start).encode()?;
    for frame in &prefix.frames {
        body.extend(frame.encode()?);
    }
    let response = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v7")
        .body(body)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    codec::frames(&response.bytes().await?)
}
