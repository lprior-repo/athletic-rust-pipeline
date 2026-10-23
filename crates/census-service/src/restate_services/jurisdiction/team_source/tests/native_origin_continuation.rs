use super::{key, ledger, request};
use crate::restate_services::national::publication_tests::native_restate::{
    endpoint, node::NativeNode,
};
use restate_sdk::prelude::{Endpoint, HttpServer};
use std::time::Duration;

mod controlled_capture;
mod exercise;
mod fixture;
mod parent;
mod profiles;
mod receipts;
mod transport;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[ignore = "explicit native Restate integration test; see docs/VERIFICATION-EVIDENCE.md"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_origin_held_continuation_completes_same_source_without_rewriting_original(
) -> TestResult {
    let fixture = fixture::Fixture::new().await?;
    let mut node = NativeNode::new()?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(60))
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
        let outcome = tokio::time::timeout(Duration::from_secs(180), async {
            node.wait_ready(&client).await?;
            endpoint::register_named_endpoint(
                &client,
                &node,
                address,
                "TeamsSource",
                "continue_origin_held",
            )
            .await?;
            exercise::run(&fixture, &client, &mut node).await
        })
        .await;
        let stopped = node.stop().await;
        let endpoint_stopped = stop_tx.send(());
        outcome??;
        stopped?;
        endpoint_stopped.map_err(|_| "TeamsSource native endpoint lost shutdown owner")?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let ((), outcome) = tokio::join!(server, assertions);
    let drained = fixture.region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    outcome
}
