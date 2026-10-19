use super::NationalCensus;
use census_store::clock::Clock;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

mod assertion;
use assertion::{assert_published, publication_services};
mod binding;
pub(crate) mod codec;
mod external;
mod fixtures;
pub(super) mod journal;
mod restricted;

mod native_restate;
mod resume;
mod transport;
mod workbook_endpoint;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct FixtureClock;

impl Clock for FixtureClock {
    fn today(&self) -> String {
        "2026-10-12".to_owned()
    }

    fn today_iso8601(&self) -> String {
        "2026-10-12T00:00:00Z".to_owned()
    }

    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    ExternalControl,
    Address(binding::Scenario),
    Complete,
    ManagedComplete,
    ManagedFailedStage,
    SelectedFailedStage,
    ManagedRefusedSeal,
    ManagedCompletedResume,
    ManagedExhaustedResume,
    FailedStage(&'static str),
    RefusedSeal,
    PartialContacts,
    UnfinishedRosters,
    CompletedResume,
    ExhaustedResume,
    InvalidResume(&'static str),
}
#[tokio::test]
async fn selected_workbook_endpoint_runs_the_reserved_handler_with_its_bound_profile() -> TestResult
{
    workbook_endpoint::exercise().await
}

#[tokio::test]
async fn current_fixture_exercises_external_replay_driver_without_historical_claim() -> TestResult {
    exercise(Scenario::ExternalControl).await
}

#[tokio::test]
async fn managed_national_publication_binds_one_original_request_and_one_bundle() -> TestResult {
    exercise(Scenario::ManagedComplete).await
}

#[tokio::test]
async fn managed_national_workbook_retry_preserves_the_same_call_and_request() -> TestResult {
    exercise(Scenario::ManagedFailedStage).await
}

#[tokio::test]
async fn selected_national_workbook_retry_preserves_the_same_profile_request() -> TestResult {
    exercise(Scenario::SelectedFailedStage).await
}

#[tokio::test]
async fn managed_national_seal_refusal_never_rebuilds_its_bundle() -> TestResult {
    exercise(Scenario::ManagedRefusedSeal).await
}

#[tokio::test]
async fn managed_national_resume_preserves_source_keys_and_original_request() -> TestResult {
    exercise(Scenario::ManagedCompletedResume).await
}

#[tokio::test]
async fn managed_national_resume_cannot_reset_exhausted_source_attempts() -> TestResult {
    exercise(Scenario::ManagedExhaustedResume).await
}

#[tokio::test]
async fn completed_national_run_reaches_publication_without_manual_verb() -> TestResult {
    exercise(Scenario::Complete).await
}

#[tokio::test]
async fn failed_workbook_stage_resumes_the_existing_national_invocation() -> TestResult {
    exercise(Scenario::FailedStage("Workbook")).await
}

#[tokio::test]
async fn unready_seal_resumes_the_original_publication_call() -> TestResult {
    exercise(Scenario::FailedStage("Census")).await
}

#[tokio::test]
async fn refused_seal_refreshes_measurement_without_new_publication_jobs() -> TestResult {
    exercise(Scenario::RefusedSeal).await
}

#[tokio::test]
async fn incomplete_contacts_do_not_release_publication() -> TestResult {
    exercise(Scenario::PartialContacts).await
}

#[tokio::test]
async fn unfinished_jurisdiction_rosters_do_not_release_publication() -> TestResult {
    exercise(Scenario::UnfinishedRosters).await
}

#[tokio::test]
async fn completed_national_run_redrives_same_source_keys_and_replays_without_duplicate_calls(
) -> TestResult {
    exercise(Scenario::CompletedResume).await
}

#[tokio::test]
async fn national_resume_inherits_exhausted_source_attempts_and_never_reserves_attempt_four(
) -> TestResult {
    exercise(Scenario::ExhaustedResume).await
}

#[tokio::test]
async fn national_resume_rejects_noncanonical_or_unbounded_execution_keys() -> TestResult {
    for suffix in ["0", "01", "-1", "4294967296", "1:resume:2"] {
        exercise(Scenario::InvalidResume(suffix)).await?;
    }
    Ok(())
}
#[tokio::test]
async fn restricted_national_run_publishes_only_its_five_states() -> TestResult {
    restricted::exercise(crate::restate_services::PublicationContract::LegacySeparateV1).await
}

#[tokio::test]
async fn managed_national_bundle_preserves_the_selected_five_states() -> TestResult {
    restricted::exercise(crate::restate_services::PublicationContract::ManagedBundleV2).await
}

async fn exercise(scenario: Scenario) -> TestResult {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(store, Arc::clone(&region));
    let endpoint = Endpoint::builder()
        .bind(NationalCensus::new(Arc::new(FixtureClock), jobs))
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            resume::run_scenario(scenario, &url),
        )
        .await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost its shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

async fn resume_stage(
    journal: &mut journal::Journal,
    client: &reqwest::Client,
    url: &str,
    stage: &str,
) -> TestResult {
    let held = journal
        .held
        .clone()
        .ok_or("national finished without awaiting its publication stage")?;
    check!(eq; held.text(1)?, stage);
    check!(eq; journal.output, None);
    let body = journal.request_body()?;
    let calls = journal.calls.clone();
    let retry = journal.step(client, url).await?;
    check!(eq; journal.request_body()?, body);
    check!(eq; journal.calls, calls);
    check!(retry.iter().any(|entry| entry.kind == 0x0001));
    check!(!retry.iter().any(|entry| entry.kind == 0x0003));
    journal.release()?;
    journal.drive(client, url).await?;
    check!(eq; journal.calls.iter().filter(|call| call == &&held).count(), 1);
    Ok(())
}

async fn refresh_seal(
    journal: &mut journal::Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    let first = journal
        .held
        .clone()
        .ok_or("initial seal measurement absent")?;
    journal.release_refusal()?;
    journal.drive(client, url).await?;
    let next = journal
        .held
        .as_ref()
        .ok_or("refusal failed to release a fresh measurement")?;
    check!(eq; first.text(6)?, format!("{}:seal:0", journal.identity));
    check!(eq; next.text(6)?, format!("{}:seal:1", journal.identity));
    check!(eq; next.bytes(3)?, first.bytes(3)?);
    let refusal = journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0403 && entry.text(1).is_ok_and(|key| key == "publication_refusal")
        })
        .ok_or("parent did not retain the observed terminal refusal")?;
    let bytes = codec::field(refusal.bytes(3)?, 1)?.ok_or("refusal state value missing")?;
    check!(eq; serde_json::from_slice::<String>(bytes)?, "the census seal refused: jurisdiction sweeps remain owed");
    resume_stage(journal, client, url, "Census").await?;
    Ok(())
}
