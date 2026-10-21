use super::{codec, fixture::Fixture, journal, key, ledger, Frame, TestResult};
use crate::restate_services::national::publication_tests::codec::field;
use crate::restate_services::wire::{JurisdictionState, TeamsFailure, TeamsStage};
use census_domain::model::serialized_digest;
use census_domain::national_census::JurisdictionOwed;
use census_store::Table;
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

mod prefix;
mod state;
mod verify;

#[tokio::test]
async fn actual_pass_replays_controlled_old_prefix_then_consumes_post_roster_continuation(
) -> TestResult {
    exercise("settled_success", false).await
}

#[tokio::test]
async fn actual_pass_replays_controlled_old_prefix_preserving_other_settled_failure() -> TestResult
{
    exercise("settled_failure", false).await
}

#[tokio::test]
async fn actual_pass_refuses_a_changed_old_call_before_reaching_continuation() -> TestResult {
    exercise("settled_success", true).await
}

async fn exercise(mode: &str, tamper: bool) -> TestResult {
    let mut fixture = Fixture::new(mode)?;
    let (clock, fetcher) = state::prepare(&mut fixture).await?;
    let before = protected(&fixture)?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/JurisdictionCensus/pass",
        listener.local_addr()?
    );
    let endpoint = Endpoint::builder().bind(fixture.owner.clone()).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        let _ = stop_rx.await;
    });
    let assertions = async {
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            verify::run(&fixture, &url, mode, tamper),
        )
        .await;
        stop_tx
            .send(())
            .map_err(|_| "actual pass endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = fixture.region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    result?;
    check!(eq; clock.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(eq; protected(&fixture)?, before);
    Ok(())
}

fn protected(fixture: &Fixture) -> TestResult<(u64, u64, u64, String, serde_json::Value)> {
    let snapshot = fixture.store.snapshot();
    Ok((
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        snapshot.tables_digest(&Table::ALL)?,
        fixture.snapshot()?,
    ))
}
