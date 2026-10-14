use crate::http_retry_fixture::{check_refused_stream, observe_refusals, TestResult};
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn reqwest_default_retries_refused_stream() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/refused", listener.local_addr()?);
    let client = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .build()?;
    let (requests, outcome) = observe_refusals(listener, client.get(&url).send()).await?;
    check!(eq; requests, [1, 3, 5]);
    let error = outcome
        .err()
        .ok_or("default transport unexpectedly succeeded")?;
    check_refused_stream(&error)
}

#[tokio::test]
async fn http_transport_does_not_retry_refused_stream() -> TestResult {
    let root = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/refused", listener.local_addr()?);
    let mut fetcher = Fetcher::new(
        root.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["127.0.0.1".into()],
    )?;
    fetcher.client = super::client_builder(Arc::clone(&fetcher.destination), None)
        .http2_prior_knowledge()
        .build()?;
    let options = FetchOptions::default();
    let (requests, outcome) = observe_refusals(listener, fetcher.get(&url, &options)).await?;
    check!(eq; requests, [1]);
    let source = match outcome {
        Err(FetchError::Transport {
            url: failed,
            source,
        }) => {
            check!(eq; failed, url);
            source
        }
        other => return Err(format!("expected transport refusal: {other:?}").into()),
    };
    check_refused_stream(&source)?;
    let stats = fetcher.stats().await;
    check!(eq; stats.physical_requests(), 1);
    check!(eq;
        stats.per_host.get("127.0.0.1").ok_or("missing origin charge")?.physical_requests(),
        1
    );
    Ok(())
}
