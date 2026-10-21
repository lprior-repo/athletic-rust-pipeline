use super::{codec, fixture::TestResult, key, replay};
use crate::restate_services::jurisdiction::{JurisdictionCensus, TeamsSource};
use crate::restate_services::wire::{NationalRequest, TeamsSourceRequest};
use crate::restate_services::{CaptureExecution, Jobs};
use census_store::{clock::SystemClock, CaptureWriterV1, Store};
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn exchange(
    store: &Arc<Store>,
    request: &TeamsSourceRequest,
    writer: &Arc<CaptureWriterV1>,
    handler: &str,
    journal: &[codec::Frame],
) -> TestResult<Vec<codec::Frame>> {
    super::super::public_resources_fixture::bind(store, request)?;
    let region = Arc::new(crate::spawn::Spawner::new());
    let owner = JurisdictionCensus::new(
        Arc::clone(store),
        Arc::new(SystemClock),
        None,
        CaptureExecution::FreshQualification(Arc::clone(writer)),
        Jobs::new(Arc::clone(store), Arc::clone(&region)),
    );
    let fetcher = fetcher(store)?;
    *owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&fetcher)));
    let source = TeamsSource::new(owner, Jobs::new(Arc::clone(store), Arc::clone(&region)));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/TeamsSource/{handler}",
        listener.local_addr()?
    );
    let endpoint = Endpoint::builder().bind(source).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(Duration::from_secs(5))
            .build()?;
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            replay::invoke(&client, &url, &key(request), journal),
        )
        .await;
        stop_tx
            .send(())
            .map_err(|_| "continuation fixture lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    result
}

pub(super) fn national_request(store: &Store) -> TestResult<NationalRequest> {
    super::super::public_resources_fixture::national(store, &super::request()?)
}

pub(super) fn qualified(root: &std::path::Path) -> TestResult<(Arc<Store>, Arc<CaptureWriterV1>)> {
    Store::create_current_qualification_v3(root)?;
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let mut source = super::request()?;
    source.source = "milesplit".into();
    super::super::public_resources_fixture::bind(&store, &source)?;
    let fetcher = fetcher(&store)?;
    super::super::public_resources_fixture::declare(&store, &writer, &source, fetcher.as_ref())?;
    Ok((Arc::new(store), Arc::new(writer)))
}

fn fetcher(store: &Store) -> TestResult<Arc<census_crawl::net::Fetcher>> {
    Ok(Arc::new(
        census_crawl::net::Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_offline(true)
        .with_origin_locks(store.root().join("fixture-origin-locks")),
    ))
}
