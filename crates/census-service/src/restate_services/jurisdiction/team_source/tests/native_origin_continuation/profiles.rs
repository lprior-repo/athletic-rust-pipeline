use super::super::public_resources_fixture;
use super::{endpoint, fixture::Fixture, transport, NativeNode, TestResult};
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{Endpoint, HttpServer};
use std::time::Duration;

mod exercise;
mod parent;
mod partial;
mod source;

#[ignore = "explicit native Restate controlled owned-profile collection qualification"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_current_census_profiles_resume_exact_owned_roster_without_http() -> TestResult {
    let mut request = super::request()?;
    request.jurisdiction.jurisdiction = UsJurisdiction::Michigan;
    let fixture = Fixture::new_for(request).await?;
    let expected = source::install(&fixture).await?;
    let mut node = NativeNode::new()?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(300))
        .build()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let endpoint = Endpoint::builder()
        .bind(fixture.source())
        .bind(fixture.owner.clone())
        .bind(fixture.national())
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    node.start()?;
    let assertions = async {
        let outcome = tokio::time::timeout(Duration::from_secs(600), async {
            node.wait_ready(&client).await?;
            endpoint::register_named_endpoint(
                &client,
                &node,
                address,
                "JurisdictionCensus",
                "public_profiles",
            )
            .await?;
            exercise::run(&fixture, &client, &mut node, &expected).await
        })
        .await;
        let stopped = node.stop().await;
        let endpoint_stopped = stop_tx.send(());
        outcome??;
        stopped?;
        endpoint_stopped.map_err(|_| "profile native endpoint lost shutdown owner")?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let ((), outcome) = tokio::join!(server, assertions);
    let drained = fixture.region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    outcome
}
