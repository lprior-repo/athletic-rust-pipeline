use super::{key, ledger, request, reserve, TestResult};
use crate::restate_services::JobError;
use census_store::Store;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

mod receipt_mismatch;
mod rejections;

fn journal(store: &Store) -> Result<BTreeMap<String, Option<Value>>, Box<dyn std::error::Error>> {
    store
        .journal_keys(ledger::PHASE)?
        .into_iter()
        .map(|key| {
            let payload = store.journal_payload(ledger::PHASE, &key)?;
            Ok((key, payload))
        })
        .collect()
}

#[test]
fn registration_retains_original_identity_without_attempts_after_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;

    let identity = ledger::register(&store, &operation, &request)?;
    let expected = serde_json::to_value(&identity)?;

    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/identity"))?,
    Some(expected.clone()));
    check!(eq; store.journal_keys(ledger::PHASE)?,
    HashSet::from([format!("{operation}/identity")]));
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    json!({"status": "unsettled", "progress": []}));

    let before = journal(&store)?;
    let replay = ledger::register(&store, &operation, &request)?;
    check!(eq; serde_json::to_value(replay)?, expected);
    check!(eq; journal(&store)?, before);

    drop(store);

    request.observed_on = "2026-10-03".to_string();
    request.jurisdiction.source_parallelism = 8;

    let store = Store::open(root.path())?;
    let replay = ledger::register(&store, &operation, &request)?;
    check!(eq; replay.observed_on, "2026-10-02");
    check!(eq; serde_json::to_value(replay)?, expected);
    check!(eq; journal(&store)?, before);
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    json!({"status": "unsettled", "progress": []}));

    Ok(())
}

fn expected_exhausted() -> Value {
    json!({
        "status": "exhausted",
        "attempts": 3,
        "last_failure": "known failure 3",
        "progress": (1..=3).map(|n| json!({
            "status": "transient",
            "attempt": n,
            "outcome": null,
            "message": format!("known failure {n}")
        })).collect::<Vec<_>>()
    })
}

#[test]
fn registration_replay_preserves_three_transient_reservations_across_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);

    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        let registered = ledger::register(&store, &operation, &request)?;
        check!(eq; registered.observed_on, "2026-10-02");
        let before = journal(&store)?;
        ledger::register(&store, &operation, &request)?;
        check!(eq; journal(&store)?, before);

        let (attempt, observed_on) = reserve(&store, &request)?;
        check!(eq; (attempt, observed_on), (expected, "2026-10-02".to_string()));

        let finished = ledger::finish(
            &store,
            &operation,
            attempt,
            Err(JobError::Transient {
                message: format!("known failure {attempt}"),
            })
            .into(),
        )?;

        match finished {
            ledger::Completion::Retry(JobError::Transient { message }) if expected < 3 => {
                check!(eq; message, format!("known failure {expected}"));
            }
            ledger::Completion::Settled(outcome) if expected == 3 => {
                check!(eq; serde_json::to_value(outcome)?, expected_exhausted());
            }
            _ => return Err("unexpected settlement transition".into()),
        }

        store.flush()?;
        request.observed_on = "2026-10-03".to_string();
    }

    let store = Store::open(root.path())?;
    let registered = ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;

    let ledger::Admission::Settled(outcome) =
        ledger::begin(&store, &operation, &request, &registered)?
    else {
        return Err("exhausted source was physically runnable".into());
    };

    check!(eq; serde_json::to_value(outcome)?, expected_exhausted());
    check!(eq; journal(&store)?, before);
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/attempt/4/reserved"))?,
    None);

    Ok(())
}

#[test]
fn registration_replay_preserves_unknown_reservations_without_claiming_exhaustion() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);

    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        let registered = ledger::register(&store, &operation, &request)?;
        check!(eq; registered.observed_on, "2026-10-02");
        let before = journal(&store)?;
        ledger::register(&store, &operation, &request)?;
        check!(eq; journal(&store)?, before);
        check!(eq; reserve(&store, &request)?,
        (expected, "2026-10-02".to_string()));
        store.flush()?;
        request.observed_on = "2026-10-03".to_string();
    }

    let store = Store::open(root.path())?;
    let registered = ledger::register(&store, &operation, &request)?;
    let ledger::Admission::Settled(outcome) =
        ledger::begin(&store, &operation, &request, &registered)?
    else {
        return Err("unknown acquisition became runnable".into());
    };
    check!(eq; serde_json::to_value(outcome)?,
    json!({
        "status": "interrupted",
        "attempts": 3,
        "message": "teams source final reservation has no recorded outcome",
        "progress": [
            {"status": "unknown", "attempt": 1},
            {"status": "unknown", "attempt": 2},
            {"status": "unknown", "attempt": 3}
        ]
    }));
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/attempt/4/reserved"))?,
    None);
    Ok(())
}
