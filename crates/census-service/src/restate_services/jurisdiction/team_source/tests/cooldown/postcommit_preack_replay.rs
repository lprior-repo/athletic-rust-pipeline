use super::super::super::{key, ledger};
use super::super::request;
use super::fixture::{Driver, FixedSequenceClock as Clock, TestResult};
use crate::restate_services::national::publication_tests::codec;
use crate::restate_services::wire::TeamsSourceRequest;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{Store, Table};
use std::sync::Arc;
fn ihsa_request() -> TestResult<TeamsSourceRequest> {
    let mut current = request()?;
    current.source = "ihsa".into();
    current.jurisdiction.jurisdiction = census_domain::UsJurisdiction::Illinois;
    Ok(current)
}
fn controlled() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "ihsa",
        "api.ihsa.org",
        AccessBlockKind::Forbidden,
        403,
        "2026-10-14T11:30:00Z",
        "controlled finite host condition for postcommit preack replay",
    )
    .with_retry_after(Some(1800))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".into()))
}
#[tokio::test]
async fn replay_after_outcome_before_run_ack_consumes_retained_retry() -> TestResult {
    let root = tempfile::tempdir()?;
    let current = ihsa_request()?;
    let operation = key(&current);
    let store = Arc::new(Store::open(root.path())?);
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    let clock = Arc::new(Clock::new_repeating(
        "2026-10-14T11:30:00Z",
        2,
        "2026-10-14T12:00:01Z",
    ));
    let mut driver = Driver::new(&current)?;
    let outcome_one = format!("{operation}/attempt/1/outcome");
    let reserved_one = format!("{operation}/attempt/1/reserved");
    let reserved_two = format!("{operation}/attempt/2/reserved");
    for _ in 0..8 {
        let events = driver.step(&store, &clock, &current).await?;
        check!(events.sleep.is_none() && events.failure.is_none() && events.output.is_none());
        if store.journal_contains(ledger::PHASE, &outcome_one)? {
            break;
        }
    }
    check!(store.journal_contains(ledger::PHASE, &outcome_one)?);
    check!(store.journal_contains(ledger::PHASE, &reserved_one)?);
    check!(!store.journal_contains(ledger::PHASE, &reserved_two)?);
    check!(driver
        .run_values
        .iter()
        .any(|value| *value == serde_json::Value::String("Proceed".to_string())));
    check!(driver
        .run_values
        .iter()
        .any(|value| value.get("request_digest").is_some()));
    let retained = driver
        .run_values
        .iter()
        .rev()
        .find(|value| value.get("Retry").is_some())
        .cloned()
        .ok_or("missing retained retry")?;
    let outcome_before = store
        .snapshot()
        .journal_payload(ledger::PHASE, &outcome_one)?
        .ok_or("missing attempt one outcome")?;
    let reserved_before = store
        .snapshot()
        .journal_payload(ledger::PHASE, &reserved_one)?
        .ok_or("missing attempt one reservation")?;
    check!(eq; ledger::durable_attempts(store.as_ref())?.reserved, 1);
    let withheld_at = driver
        .journal
        .iter()
        .rposition(|frame| frame.kind == 0x8011)
        .ok_or("missing run completion to withhold")?;
    let withheld = driver.journal.remove(withheld_at);
    let inner = codec::field(&withheld.payload, 5)?.ok_or("missing withheld inner")?;
    let content = codec::field(inner, 1)?.ok_or("missing withheld content")?;
    let withheld_value: serde_json::Value = serde_json::from_slice(content)?;
    check!(eq; withheld_value, retained);
    check!(driver.journal.iter().any(|frame| frame.kind == 0x0411));
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    let row = controlled();
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&row))?;
    batch.commit()?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![row.clone()]);
    check!(eq; store.snapshot().journal_payload(ledger::PHASE, &outcome_one)?, Some(outcome_before.clone()));
    check!(eq; store.snapshot().journal_payload(ledger::PHASE, &reserved_one)?, Some(reserved_before.clone()));
    let mut slept = None;
    for _ in 0..8 {
        let events = driver.step(&store, &clock, &current).await?;
        check!(!store.journal_contains(ledger::PHASE, &reserved_two)?);
        check!(driver.run_values.iter().any(|value| value == &retained));
        if let Some(frame) = events.sleep {
            slept = Some(frame);
            break;
        }
        check!(events.failure.is_none() && events.output.is_none());
    }
    let slept = slept.ok_or("missing new gate sleep")?;
    check!(eq; slept.kind, 0x040C);
    check!(eq; clock.calls(), 2);
    check!(driver
        .decisions
        .iter()
        .any(|value| value.get("Sleep").is_some()));
    let sleep_value = driver
        .decisions
        .iter()
        .rev()
        .find(|value| value.get("Sleep").is_some())
        .cloned()
        .ok_or("missing sleep decision")?;
    check!(eq; sleep_value.get("Sleep").and_then(|inner| inner.get("milliseconds")), Some(&serde_json::json!(1800000)));
    check!(!store.journal_contains(ledger::PHASE, &reserved_two)?);
    check!(eq; ledger::durable_attempts(store.as_ref())?.reserved, 1);
    check!(eq; store.snapshot().journal_payload(ledger::PHASE, &outcome_one)?, Some(outcome_before));
    check!(eq; store.snapshot().journal_payload(ledger::PHASE, &reserved_one)?, Some(reserved_before));
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![row]);
    Ok(())
}
