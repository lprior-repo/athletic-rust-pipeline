use super::super::super::{key, ledger};
use super::super::request;
use super::fixture::{Driver, FixedSequenceClock as Clock, TestResult};
use crate::restate_services::wire::TeamsSourceRequest;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{Store, Table};
use std::sync::Arc;

const MAX_STEPS: usize = 8;

fn ihsa_request() -> TestResult<TeamsSourceRequest> {
    let mut request = request()?;
    request.source = "ihsa".into();
    request.jurisdiction.jurisdiction = census_domain::UsJurisdiction::Illinois;
    Ok(request)
}

fn controlled_late_condition() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "ihsa",
        "api.ihsa.org",
        AccessBlockKind::Forbidden,
        403,
        "2026-10-14T11:30:00Z",
        "controlled late host condition inserted after the first gate Proceed; not a genuine HTTP 403 response",
    )
    .with_retry_after(Some(1_800))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".into()))
}

#[tokio::test]
async fn late_condition_after_first_gate_proceed_sleeps_before_attempt_two_and_replays_attempt_one(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let request = ihsa_request()?;
    let operation = key(&request);
    let store = Arc::new(Store::open(root.path())?);
    check!(store
        .scan::<SourceAccessCondition>(Table::SourceAccess)?
        .is_empty());
    let clock = Arc::new(Clock::new_repeating(
        "2026-10-14T11:30:00Z",
        2,
        "2026-10-14T12:00:01Z",
    ));
    let mut driver = Driver::new(&request)?;

    let mut first_gate_journaled = false;
    for _ in 0..MAX_STEPS {
        let events = driver.step(&store, &clock, &request).await?;
        check!(events.sleep.is_none() && events.failure.is_none() && events.output.is_none());
        if clock.calls() == 1 {
            first_gate_journaled = true;
            break;
        }
    }
    check!(
        first_gate_journaled,
        "the actual SDK handler never journaled its first recorded-access gate decision"
    );
    check!(eq; clock.calls(), 1);
    check!(
        driver.journal.iter().any(|frame| frame.kind == 0x8011),
        "the first gate Run completion is absent from the retained SDK journal"
    );
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/1/reserved"))?);
    let journal_at_first_gate = driver.journal.clone();

    let row = controlled_late_condition();
    let mut batch = store.write_batch();
    batch.merge_source_access(std::slice::from_ref(&row))?;
    batch.commit()?;
    check!(eq; driver.journal, journal_at_first_gate);
    check!(
        eq;
        store.scan::<SourceAccessCondition>(Table::SourceAccess)?,
        vec![row.clone()]
    );

    let reserved_one = format!("{operation}/attempt/1/reserved");
    let outcome_one = format!("{operation}/attempt/1/outcome");
    let mut attempt_one_acknowledged = false;
    for _ in 0..MAX_STEPS {
        if store
            .snapshot()
            .journal_payload(ledger::PHASE, &outcome_one)?
            .is_some()
        {
            attempt_one_acknowledged = true;
            break;
        }
        let events = driver.step(&store, &clock, &request).await?;
        check!(
            events.sleep.is_none() && events.output.is_none(),
            "the handler slept or settled before acknowledging attempt one"
        );
    }
    check!(
        attempt_one_acknowledged,
        "attempt one was not acknowledged within the step bound"
    );
    let reserved_one_receipt = store
        .snapshot()
        .journal_payload(ledger::PHASE, &reserved_one)?
        .ok_or("attempt one reservation receipt is absent")?;
    let outcome_one_receipt = store
        .snapshot()
        .journal_payload(ledger::PHASE, &outcome_one)?
        .ok_or("attempt one outcome receipt is absent")?;
    check!(eq; ledger::durable_attempts(store.as_ref())?.reserved, 1);
    check!(
        eq;
        store.scan::<SourceAccessCondition>(Table::SourceAccess)?,
        vec![row.clone()]
    );
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    let ledger_after_attempt_one = store.journal_keys(ledger::PHASE)?;

    let mut late_gate_run_journaled = false;
    let mut sleep = None;
    for _ in 0..MAX_STEPS {
        let run_commands = driver
            .journal
            .iter()
            .filter(|frame| frame.kind == 0x0411)
            .count();
        let events = driver.step(&store, &clock, &request).await?;
        if driver
            .journal
            .iter()
            .filter(|frame| frame.kind == 0x0411)
            .count()
            > run_commands
        {
            late_gate_run_journaled = true;
        }
        check!(
            !store.journal_contains(
                ledger::PHASE,
                &format!("{operation}/attempt/2/reserved")
            )?,
            "the cached first-gate Proceed reserved a second attempt before the late condition slept"
        );
        check!(
            events.output.is_none(),
            "the handler settled instead of gating the late condition"
        );
        if let Some(command) = events.sleep {
            sleep = Some(command);
            break;
        }
        check!(
            events.failure.is_none(),
            "the handler failed before observing the late condition at a new gate position"
        );
    }
    check!(
        late_gate_run_journaled,
        "the late condition was not evaluated at a new journaled gate position"
    );
    let sleep =
        sleep.ok_or("the late condition produced no durable sleep within the step bound")?;
    check!(eq; clock.calls(), 2);
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    check!(eq; store.journal_keys(ledger::PHASE)?, ledger_after_attempt_one);
    check!(
        eq;
        store.scan::<SourceAccessCondition>(Table::SourceAccess)?,
        vec![row.clone()]
    );

    driver.complete_sleep(&sleep)?;
    let ledger_after_sleep = store.journal_keys(ledger::PHASE)?;
    let mut attempt_two = false;
    for _ in 0..MAX_STEPS {
        let events = driver.step(&store, &clock, &request).await?;
        check!(events.sleep.is_none() && events.failure.is_none() && events.output.is_none());
        if store
            .snapshot()
            .journal_payload(ledger::PHASE, &format!("{operation}/attempt/2/outcome"))?
            .is_some()
        {
            attempt_two = true;
            break;
        }
        check!(
            !store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?,
            "attempt two was reserved without first executing the post-sleep gate"
        );
        check!(eq; store.journal_keys(ledger::PHASE)?, ledger_after_sleep);
    }
    check!(
        attempt_two,
        "attempt two was not permitted within the step bound"
    );
    check!(
        eq;
        clock.calls(),
        3,
        "the post-sleep gate did not read a fresh clock"
    );
    check!(store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    check!(eq; ledger::durable_attempts(store.as_ref())?.reserved, 2);
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/3/reserved"))?);

    check!(
        eq;
        store
            .snapshot()
            .journal_payload(ledger::PHASE, &reserved_one)?,
        Some(reserved_one_receipt)
    );
    check!(
        eq;
        store
            .snapshot()
            .journal_payload(ledger::PHASE, &outcome_one)?,
        Some(outcome_one_receipt)
    );
    check!(
        eq;
        store.scan::<SourceAccessCondition>(Table::SourceAccess)?,
        vec![row]
    );
    Ok(())
}
