use super::*;
use crate::restate_services::results_arms::ResultsSourceRows;
use crate::spawn::TaskReport;
use census_domain::model::{AcquisitionFailureV1, CaptureProtocolVersion};
use census_store::{AcquisitionTerminal, ImmutableAttemptResult};
use std::time::Duration;

mod fixture;
mod protocol;
mod v2;
use fixture::Fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "current_thread")]
async fn historical_single_run_success_journal_refuses_without_releasing_more_work() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.seed_failed_attempt()?;
    require_not_dispatched(&fixture)?;
    let resources = fixture.region.coordinator().remaining();
    let expected = recorded_outcome();
    let journal = protocol::recorded(&fixture, &expected)?;
    let before = snapshot(&fixture).await?;
    let f = &fixture;
    let result = protocol::serve(&fixture, |client, url| async move {
        for _ in 0..2 {
            let frames = protocol::invoke_frames(&client, &url, f, &journal).await?;
            require_historical_refusal(&frames)?;
            check!(eq; snapshot(f).await?, before);
            readmit_same_key(f).await?;
            check!(eq; f.region.coordinator().remaining(), resources);
        }
        Ok(())
    })
    .await;
    finish(&fixture, result, 0, resources).await
}

#[tokio::test(flavor = "current_thread")]
async fn capture_results_stage_finalizes_offline_attempt_with_only_stage_permit_then_replays(
) -> TestResult {
    let fixture = Fixture::new()?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = protocol::serve(&fixture, |client, url| async move {
        let mut journal = protocol::input(f)?;
        let frames = protocol::invoke(&client, &url, f, &journal).await?;
        let plan = v2::protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        check!(serde_json::from_slice::<serde_json::Value>(&plan)?
            .get("pins")
            .is_some());
        readmit_same_key(f).await?;
        let frames = protocol::invoke(&client, &url, f, &journal).await?;
        let bytes = v2::protocol::acknowledge(&mut journal, &frames, "results-v2-execute")?;
        let outcome = serde_json::from_slice(&bytes)?;
        require_offline_outcome(&outcome)?;
        require_not_dispatched(f)?;
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        check!(eq; f.fetcher.stats().await.cache_hits, 1);
        readmit_same_key(f).await?;
        let before = snapshot(f).await?;
        let frames = protocol::invoke(&client, &url, f, &journal).await?;
        protocol::require_replay_only(&frames)?;
        protocol::require_saved_outcome(&frames, f, &outcome)?;
        check!(eq; protocol::output(&frames)?, serde_json::to_vec(&outcome)?);
        check!(eq; snapshot(f).await?, before);
        readmit_same_key(f).await?;
        check!(eq; f.region.coordinator().remaining(), resources);
        Ok(())
    })
    .await;
    finish(&fixture, result, 2, resources).await
}

#[tokio::test(flavor = "current_thread")]
async fn historical_single_run_failure_journal_refuses_without_releasing_more_work() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.seed_failed_attempt()?;
    require_not_dispatched(&fixture)?;
    let resources = fixture.region.coordinator().remaining();
    let journal = protocol::recorded_failure(&fixture)?;
    let before = snapshot(&fixture).await?;
    let f = &fixture;
    let result = protocol::serve(&fixture, |client, url| async move {
        let frames = protocol::invoke_frames(&client, &url, f, &journal).await?;
        require_historical_refusal(&frames)?;
        check!(eq; snapshot(f).await?, before);
        readmit_same_key(f).await?;
        check!(eq; f.region.coordinator().remaining(), resources);
        Ok(())
    })
    .await;
    finish(&fixture, result, 0, resources).await
}

fn require_historical_refusal(
    frames: &[crate::restate_services::national::publication_tests::codec::Frame],
) -> TestResult {
    let error = frames
        .iter()
        .find(|frame| frame.kind == 0x0002)
        .ok_or("historical unnamed Results run did not refuse")?;
    let message = error.text(2)?;
    check!(message.contains("mismatch") && message.contains("name:  != results-v2-plan"));
    check!(eq; frames.iter().filter(|frame| matches!(frame.kind, 0x0005 | 0x0401 | 0x0403 | 0x0411)).count(), 0);
    Ok(())
}

async fn readmit_same_key(fixture: &Fixture) -> TestResult {
    let admission = tokio::time::timeout(WAIT, fixture.jobs.admit_work(fixture.key()))
        .await?
        .map_err(crate::restate_services::tests::sdk_error)?;
    drop(admission);
    Ok(())
}

async fn finish(
    fixture: &Fixture,
    result: TestResult,
    accepted: u64,
    resources: crate::admission::Resources,
) -> TestResult {
    let report = fixture.region.drain(WAIT).await?;
    result?;
    check!(eq; report, TaskReport {
        accepted, completed: accepted, cancelled: 0, aborted: 0, panicked: 0,
        failed: 0, still_running: 0, timed_out: 0, remaining: 0,
    });
    check!(eq; report.accepted, report.disjoint_total());
    check!(eq; fixture.region.coordinator().remaining(), resources);
    Ok(())
}

async fn snapshot(fixture: &Fixture) -> TestResult<serde_json::Value> {
    let keys: std::collections::BTreeSet<_> = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?
        .into_iter()
        .collect();
    Ok(serde_json::json!({
        "acquisition_keys": keys,
        "acquisition_payloads": fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?,
        "source_meets": fixture.store.scan::<census_domain::model::SourceMeetRef>(census_store::Table::SourceMeets)?,
        "store_sequence": fixture.store.snapshot().sequence(),
        "fetch_stats": fixture.fetcher.stats().await,
    }))
}

fn require_not_dispatched(fixture: &Fixture) -> TestResult {
    let keys = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    check!(eq; keys.iter().filter(|key| key.starts_with("slot/") && key.ends_with("/binding")).count(), 1);
    check!(eq; keys.iter().filter(|key| key.starts_with("work/") && key.ends_with("/identity")).count(), 1);
    check!(eq; keys.iter().filter(|key| key.ends_with("/attempt/1/reserved")).count(), 1);
    check!(eq; keys.iter().filter(|key| key.contains("/dispatch/")).count(), 0);
    check!(eq; keys.iter().filter(|key| key.contains("/attempt/2/")).count(), 0);
    let terminals: Vec<_> = keys
        .iter()
        .filter(|key| key.ends_with("/terminal"))
        .collect();
    check!(eq; terminals.len(), 1);
    let key = terminals.first().ok_or("capture terminal key missing")?;
    let value = fixture
        .store
        .journal_payload(CaptureProtocolVersion::V2.phase(), key)?
        .ok_or("capture terminal missing")?;
    let terminal: ImmutableAttemptResult = serde_json::from_value(value)?;
    check!(eq; terminal.attempt().ordinal().get(), 1);
    check!(eq; terminal.terminal(), &AcquisitionTerminal::NotDispatched {
        failure: AcquisitionFailureV1::AdmissionRefused,
    });
    Ok(())
}

fn require_offline_outcome(outcome: &ResultsStageOutcome) -> TestResult {
    check!(eq; outcome.required_sources, vec!["milesplit".to_owned()]);
    check!(eq; outcome.pending, Vec::<String>::new());
    check!(eq; outcome.per_source.len(), 1);
    let source = outcome
        .per_source
        .first()
        .ok_or("MileSplit result source absent")?;
    check!(eq; source.slug, "milesplit");
    check!(eq; source.meets, 1);
    check!(eq; source.rows, None);
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Failed);
    check!(eq; source.errors, 1);
    check!(eq; source.withheld, None);
    check!(eq; source.unresolved, None);
    check!(eq; source.resolution, None);
    check!(eq; source.unfinished, vec![fixture::RESULTS_URL.to_owned()]);
    let [failure, pages] = source.notes.as_slice() else {
        return Err("offline result requires failure and discovery page notes".into());
    };
    check!(failure.contains(&format!("offline and not cached: {}", fixture::owned_url())));
    check!(eq; pages, "meet_pages_read=1");
    check!(!outcome.is_terminal());
    Ok(())
}

fn recorded_outcome() -> ResultsStageOutcome {
    ResultsStageOutcome {
        per_source: vec![ResultsSourceRows {
            slug: "milesplit".to_owned(),
            meets: 1,
            rows: None,
            disposition: census_crawl::CollectionDisposition::Failed,
            errors: 1,
            withheld: None,
            notes: vec!["retained original Results outcome".to_owned()],
            unfinished: vec![fixture::RAW_URL.to_owned()],
            unresolved: None,
            resolution: None,
        }],
        pending: Vec::new(),
        required_sources: vec!["milesplit".to_owned()],
    }
}
