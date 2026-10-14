use super::protocol as endpoint;
use super::{Fixture, ResultsStageOutcome, TestResult, WAIT};
use crate::restate_services::national::publication_tests::codec::Frame;
use census_domain::model::CaptureProtocolVersion;
use census_store::{AcquisitionTerminal, ImmutableAttemptResult, Receipt};

mod fixture;
mod protocol;
mod refusal;

const PHASE: &str = "source_acquisition_attempt_v2";

#[tokio::test(flavor = "current_thread")]
async fn completed_v2_plan_replays_the_same_closed_pin_after_lost_ack_and_later_history(
) -> TestResult {
    let fixture = Fixture::v2()?;
    let prepared = fixture::seed_prepared(&fixture)?;
    let mut alias = super::fixture::meet();
    alias.id = "selected-alias-of-725218".to_owned();
    fixture
        .store
        .append(census_store::Table::SourceMeets, &alias)?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let before_plan = acquisition(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let plan = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        require_pinned_plan(&plan, &prepared)?;
        check!(eq; acquisition(f)?, before_plan);
        super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let original = protocol::leave_unacknowledged(&mut journal, &frames, "results-v2-execute")?;
        let original_bytes = protocol::proposal(&frames)?.bytes(14)?.to_vec();
        let original_receipt = require_closed(f, &prepared)?;
        require_failed_rows(&original, 2, &original_receipt)?;
        super::readmit_same_key(f).await?;
        fixture::publish_then_corrupt(f, &prepared)?;
        fixture::change_candidates_and_discovery(f)?;
        let before_replay = snapshot(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        check!(eq; protocol::proposal(&frames)?.bytes(14)?, original_bytes.as_slice());
        let replay = protocol::acknowledge_reexecution(&mut journal, &frames)?;
        check!(eq; replay, original);
        check!(eq; require_closed(f, &prepared)?, original_receipt);
        check!(eq; snapshot(f).await?, before_replay);
        require_no_third_attempt(f)?;
        super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &original)?;
        check!(eq; endpoint::output(&frames)?, serde_json::to_vec(&original)?);
        check!(eq; snapshot(f).await?, before_replay);
        super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::finish(&fixture, result, 3, resources).await
}

fn require_pinned_plan(bytes: &[u8], prepared: &fixture::Prepared) -> TestResult {
    let plan: serde_json::Value = serde_json::from_slice(bytes)?;
    let pins = plan
        .get("pins")
        .and_then(serde_json::Value::as_array)
        .ok_or("plan pins absent")?;
    check!(eq; pins.len(), 1);
    let pin: census_domain::model::PreparedRecoveryTarget =
        serde_json::from_value(pins.first().ok_or("pin absent")?.clone())?;
    check!(eq; pin.attempt(), &prepared.attempt);
    check!(eq; plan["selection_usage"]["distinct_targets"].as_u64(), Some(1));
    let routes = plan
        .get("routes")
        .and_then(serde_json::Value::as_array)
        .ok_or("routes absent")?;
    check!(eq; routes.len(), 2);
    check!(eq; routes.first(), routes.get(1));
    Ok(())
}

fn require_closed(fixture: &Fixture, prepared: &fixture::Prepared) -> TestResult<Receipt> {
    let key = format!("work/{}/attempt/1/terminal", prepared.attempt.work_id());
    let value = fixture
        .store
        .journal_payload(PHASE, &key)?
        .ok_or("closed terminal absent")?;
    let terminal: ImmutableAttemptResult = serde_json::from_value(value)?;
    check!(eq; terminal.attempt(), &prepared.attempt);
    check!(matches!(
        terminal.terminal(),
        AcquisitionTerminal::PreparedRetentionFailed { .. }
    ));
    let receipt = fixture
        .store
        .receipt(&format!("{PHASE}/{key}"))?
        .ok_or("closed receipt absent")?;
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    Ok(receipt)
}

fn require_failed_rows(
    outcome: &ResultsStageOutcome,
    meets: usize,
    receipt: &Receipt,
) -> TestResult {
    let source = outcome.per_source.first().ok_or("source outcome absent")?;
    check!(eq; source.meets, meets);
    check!(eq; source.rows, None);
    check!(!outcome.is_terminal());
    check!(source
        .notes
        .iter()
        .any(|note| note.contains("retention_failed") && note.contains(&receipt.digest)));
    check!(source
        .unfinished
        .iter()
        .any(|url| url == super::fixture::RESULTS_URL));
    Ok(())
}

fn acquisition(fixture: &Fixture) -> TestResult<serde_json::Value> {
    check!(eq; CaptureProtocolVersion::V2.phase(), PHASE);
    Ok(
        serde_json::json!({ "keys":fixture.store.journal_keys(PHASE)?, "payloads":fixture.store.journal_payloads(PHASE)? }),
    )
}

async fn snapshot(fixture: &Fixture) -> TestResult<serde_json::Value> {
    Ok(
        serde_json::json!({ "acquisition":acquisition(fixture)?, "sequence":fixture.store.snapshot().sequence(), "fetch":fixture.fetcher.stats().await }),
    )
}

fn require_no_third_attempt(fixture: &Fixture) -> TestResult {
    let keys = fixture.store.journal_keys(PHASE)?;
    check!(keys.iter().any(|key| key.ends_with("/attempt/2/terminal")));
    check!(!keys.iter().any(|key| key.contains("/attempt/3/")));
    Ok(())
}
