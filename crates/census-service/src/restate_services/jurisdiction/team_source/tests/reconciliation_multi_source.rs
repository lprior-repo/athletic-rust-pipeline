use super::{key, ledger, request};
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::national::publication_tests::codec::{self, Frame};
use crate::restate_services::wire::{
    JurisdictionState, SourcePlan, StageOutcome, TeamsContinuationOutcome, TeamsFailure,
    TeamsReconciliationRequest, TeamsSourceOutcome, TeamsSourceRequest, TeamsStage,
};
use crate::restate_services::{CaptureExecution, JobError, Jobs};
use census_crawl::CollectionDisposition;
use census_domain::model::{serialized_digest as digest, CensusRun, GradYear, RunManifest};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{clock::SystemClock, Store};
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod fixture;
mod journal;

use fixture::{seed, snapshot, Fixture};
use journal::drive;

#[tokio::test]
async fn reconciliation_preserves_other_source_failure_and_replays_exactly() -> TestResult {
    exercise(true, "settled").await
}

#[tokio::test]
async fn reconciliation_preserves_other_source_success_and_plan_order() -> TestResult {
    exercise(false, "settled").await
}

#[tokio::test]
async fn invalid_parent_reconciliation_never_writes_store_or_parent_state() -> TestResult {
    for refusal in ["unselected", "unsettled", "stale"] {
        exercise(true, refusal).await?;
    }
    Ok(())
}

async fn exercise(failed: bool, mode: &str) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let mut fixture = seed(&store, failed)?;
    fixture.configure(mode)?;
    let before = snapshot(&store)?;
    let region = Arc::new(crate::spawn::Spawner::new());
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(&store), Arc::clone(&region)),
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/JurisdictionCensus/reconcile_teams",
        listener.local_addr()?
    );
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(Endpoint::builder().bind(owner).build()).serve_with_cancel(
        listener,
        async move {
            let _ = stop_rx.await;
        },
    );
    let assertions = async {
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            verify(&store, &fixture, &url, mode),
        )
        .await;
        stop_tx
            .send(())
            .map_err(|_| "reconciliation endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    result?;
    check!(eq; snapshot(&store)?, before);
    Ok(())
}

async fn verify(store: &Store, fixture: &Fixture, url: &str, mode: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut eager = codec::blob(1, b"state")?;
    eager.extend(codec::blob(
        14,
        &codec::value(&serde_json::to_vec(&fixture.state)?)?,
    )?);
    let mut journal = vec![
        Frame::new(
            0x0400,
            codec::blob(14, &codec::value(&serde_json::to_vec(&fixture.request)?)?)?,
        ),
        Frame::new(0x0407, eager),
    ];
    let (output, saved, calls) = drive(&client, url, fixture, &mut journal).await?;
    if mode != "settled" {
        check!(saved.is_empty());
        let error = codec::field(output.bytes(15)?, 2)?.ok_or("terminal failure lacks message")?;
        let expected = match mode {
            "unselected" => "teams reconciliation source plan is ambiguous",
            "unsettled" => "teams reconciliation requires every planned source to be settled",
            "stale" => "teams reconciliation parent state or plan changed",
            _ => return Err("unknown refusal".into()),
        };
        check!(eq; std::str::from_utf8(error)?, expected);
        check!(eq; calls.len(), usize::from(mode == "unsettled"));
        return Ok(());
    }
    check!(eq; calls, vec!["ihsa".to_string(), "milesplit".to_string()]);
    let [saved] = saved.as_slice() else {
        return Err("parent was not saved exactly once".into());
    };
    let saved: JurisdictionState = serde_json::from_slice(
        codec::field(saved.bytes(3)?, 1)?.ok_or("saved state missing content")?,
    )?;
    let result: TeamsStage = serde_json::from_slice(
        codec::field(output.bytes(14)?, 1)?.ok_or("output missing content")?,
    )?;
    check!(eq; digest(&saved.plan)?, digest(&fixture.state.plan)?);
    check!(saved.rosters.is_none() && saved.consolidated.is_none());
    check!(eq; digest(&saved.teams)?, digest(&result)?);
    check_result(&result, &fixture.ihsa)?;
    let before = snapshot(store)?;
    let replay = super::replay::invoke(&client, url, &fixture.state.identity, &journal).await?;
    check!(!replay
        .iter()
        .any(|frame| matches!(frame.kind, 0x0403 | 0x040D | 0x0411 | 0x0005)));
    check!(eq; replay.iter().find(|frame| frame.kind == 0x0401).ok_or("replay output absent")?.payload, output.payload);
    check!(eq; snapshot(store)?, before);
    Ok(())
}

fn check_result(result: &TeamsStage, ihsa: &TeamsSourceOutcome) -> TestResult {
    let (outcome, failed) = match result {
        TeamsStage::Completed(outcome) => (outcome.outcome(), false),
        TeamsStage::Failed(TeamsFailure::SourceFailures {
            outcome, failures, ..
        }) => {
            let [failure] = failures.as_slice() else {
                return Err("IHSA failure was lost or duplicated".into());
            };
            check!(eq; failure.source, "ihsa");
            check!(eq; serde_json::to_vec(&failure.outcome)?, serde_json::to_vec(ihsa)?);
            (outcome, true)
        }
        _ => return Err("reconciliation returned unexpected aggregate".into()),
    };
    let (records, note) = if failed {
        (7, "ihsa: ihsa retained rows")
    } else {
        (9, "ihsa: ihsa settled rows")
    };
    check!(eq; outcome.records, records);
    check!(eq; outcome.notes, vec![note, "milesplit: continued school index"]);
    check!(eq; outcome.errors, if failed { vec!["ihsa: terminal: IHSA public source refused"] } else { Vec::<&str>::new() });
    check!(outcome.unfinished.is_empty());
    check!(eq; outcome.disposition, if failed { CollectionDisposition::Partial } else { CollectionDisposition::Complete });
    Ok(())
}
