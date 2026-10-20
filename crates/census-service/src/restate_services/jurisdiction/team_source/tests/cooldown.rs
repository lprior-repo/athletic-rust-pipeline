use super::{key, ledger, replay, request};
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::JobError;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{Store, Table};
use std::sync::Arc;

mod fixture;
mod late_condition;
mod postcommit_preack_replay;
use fixture::{Driver, FixedSequenceClock as Clock, TestResult};

fn ihsa_request() -> TestResult<TeamsSourceRequest> {
    let mut request = request()?;
    request.source = "ihsa".into();
    request.jurisdiction.jurisdiction = census_domain::UsJurisdiction::Illinois;
    Ok(request)
}

fn condition() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "ihsa",
        "api.ihsa.org",
        AccessBlockKind::Forbidden,
        403,
        "2026-10-14T11:00:00Z",
        "retained IHSA 403 fixture",
    )
    .with_retry_after(Some(3600))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".into()))
}

fn seed_attempt(
    store: &Store,
    request: &TeamsSourceRequest,
    row: &SourceAccessCondition,
) -> TestResult {
    let operation = key(request);
    let registered = ledger::register(store, &operation, request)?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } = ledger::begin_at(
        store,
        &operation,
        request,
        &registered,
        ledger::baseline(store, &operation)?,
    )?
    else {
        return Err("fixture settled before attempt one".into());
    };
    check!(eq; attempt, 1);
    let finished = ledger::finish(
        store,
        &operation,
        attempt,
        ledger::Attempt::Failed(JobError::Transient {
            message: "IHSA HTTP 403 fixture".into(),
        }),
        std::slice::from_ref(row),
    )?;
    check!(
        matches!(finished, ledger::Completion::Retry(JobError::Transient { message }) if message == "IHSA HTTP 403 fixture")
    );
    Ok(())
}

fn unchanged(
    store: &Store,
    request: &TeamsSourceRequest,
    row: &SourceAccessCondition,
    attempts: u64,
) -> TestResult {
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![row.clone()]);
    check!(eq; ledger::durable_attempts(store)?.reserved, attempts);
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/3/reserved", key(request))
    )?);
    Ok(())
}

#[tokio::test]
async fn cooldown_replay_uses_fresh_clock_after_sdk_sleep_before_attempt_two() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = ihsa_request()?;
    let row = condition();
    let store = Arc::new(Store::open(root.path())?);
    seed_attempt(&store, &request, &row)?;
    let original = store.snapshot().journal_payload(
        ledger::PHASE,
        &format!("{}/attempt/1/outcome", key(&request)),
    )?;
    let clock = Arc::new(Clock::new("2026-10-14T11:30:00Z", "2026-10-14T12:00:01Z"));
    check!(eq; original, Some(serde_json::json!({
        "status": "transient", "message": "IHSA HTTP 403 fixture", "progress": null
    })));
    let mut driver = Driver::new(&request)?;
    let sleep = driver.until_sleep(&store, &clock, &request).await?;
    let decision = driver
        .decisions
        .first()
        .and_then(|value| value.get("Sleep"))
        .ok_or("missing finite cooldown decision")?;
    check!(eq; decision.get("milliseconds"), Some(&serde_json::json!(1_800_000)));
    unchanged(&store, &request, &row, 1)?;
    check!(eq; clock.calls(), 1);
    let before = driver.journal.clone();
    let pending = driver.step(&store, &clock, &request).await?;
    check!(pending.sleep.is_none() && pending.failure.is_none() && pending.output.is_none());
    check!(eq; driver.journal, before);
    check!(eq; clock.calls(), 1);
    unchanged(&store, &request, &row, 1)?;
    driver.persist(&root.path().join("sdk-journal"))?;
    store.flush()?;
    drop(store);
    let store = Arc::new(Store::open(root.path())?);
    driver.restore(&root.path().join("sdk-journal"))?;
    driver.complete_sleep(&sleep)?;
    let second = format!("{}/attempt/2/outcome", key(&request));
    for _ in 0..8 {
        let events = driver.step(&store, &clock, &request).await?;
        check!(events.sleep.is_none() && events.failure.is_none() && events.output.is_none());
        if store.journal_contains(ledger::PHASE, &second)? {
            break;
        }
    }
    check!(store.journal_contains(ledger::PHASE, &second)?);
    check!(eq; driver.decisions.last(), Some(&serde_json::json!("Proceed")));
    check!(eq; clock.calls(), 2);
    unchanged(&store, &request, &row, 2)?;
    check!(store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&request))
    )?);
    check!(eq; store.snapshot().journal_payload(ledger::PHASE, &format!("{}/attempt/1/outcome", key(&request)))?, original);
    Ok(())
}

#[tokio::test]
async fn finalized_history_beats_cached_none_without_admission_or_mutation() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let request = ihsa_request()?;
    let row = condition();
    let clock = Arc::new(Clock::new("2026-10-14T11:30:00Z", "2026-10-14T11:30:00Z"));
    let mut driver = Driver::new(&request)?;
    driver.step(&store, &clock, &request).await?;
    check!(eq; driver.run_values, vec![serde_json::Value::Null]);
    seed_attempt(&store, &request, &row)?;
    let registered = ledger::register(&store, &key(&request), &request)?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } = ledger::begin_at(
        &store,
        &key(&request),
        &request,
        &registered,
        ledger::baseline(&store, &key(&request))?,
    )?
    else {
        return Err("fixture unexpectedly settled".into());
    };
    let ledger::Completion::Settled(expected) = ledger::finish(
        &store,
        &key(&request),
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "finalized independently".into(),
        }),
        &[],
    )?
    else {
        return Err("fixture did not settle".into());
    };
    let before = store.journal_keys(ledger::PHASE)?;
    let sequence = store.snapshot().sequence();
    let actual = driver.until_output(&store, &clock, &request).await?;
    check!(eq; actual, serde_json::to_value(&expected)?);
    check!(eq; driver.decisions, vec![serde_json::json!({"Settled": expected})]);
    check!(eq; clock.calls(), 0);
    check!(eq; store.journal_keys(ledger::PHASE)?, before);
    check!(eq; store.snapshot().sequence(), sequence);
    unchanged(&store, &request, &row, 2)?;
    Ok(())
}

#[tokio::test]
async fn indefinite_condition_refuses_before_registration_and_preserves_exact_evidence(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let request = ihsa_request()?;
    let mut row = condition();
    row.kind = AccessBlockKind::HumanRequired;
    row.id = "human_required:api.ihsa.org".into();
    row.cooldown_until = None;
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&row))?;
    batch.commit()?;
    let clock = Arc::new(Clock::new("2026-10-14T11:30:00Z", "2026-10-14T11:30:00Z"));
    let mut driver = Driver::new(&request)?;
    driver.until_failure(&store, &clock, &request).await?;
    check!(store.journal_keys(ledger::PHASE)?.is_empty());
    unchanged(&store, &request, &row, 0)?;
    check!(eq; clock.calls(), 1);
    Ok(())
}

#[tokio::test]
async fn three_checks_allow_only_two_sleeps_without_reserving_an_attempt() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let request = ihsa_request()?;
    let row = condition();
    seed_attempt(&store, &request, &row)?;
    let clock = Arc::new(Clock::new("2026-10-14T11:30:00Z", "2026-10-14T11:30:00Z"));
    let mut driver = Driver::new(&request)?;
    for _ in 0..2 {
        let sleep = driver.until_sleep(&store, &clock, &request).await?;
        unchanged(&store, &request, &row, 1)?;
        driver.complete_sleep(&sleep)?;
    }
    driver.until_failure(&store, &clock, &request).await?;
    check!(eq; clock.calls(), 3);
    check!(eq; driver.decisions.len(), 3);
    unchanged(&store, &request, &row, 1)?;
    Ok(())
}
