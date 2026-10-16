use crate::{JournalReadBudget, Store, MAX_JOURNAL_VALUE_BYTES};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PHASE: &str = "attachment_envelope_test";
const KEY: &str = "record";

#[test]
fn timestamped_raw_envelope_preserves_payload_and_pinned_time() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let key = Store::journal_key(PHASE, KEY);
    let original = br#"{"key":"record","at":"2026-05-15","payload": {"value":1}}"#;
    store.journal.insert(&key, original)?;
    let held = store.snapshot();
    store.journal.insert(
        &key,
        br#"{"key":"record","at":"2026-05-16","payload":{"value":2}}"#,
    )?;
    let value = held
        .journal_entry_raw_bounded(PHASE, KEY, JournalReadBudget::new(4096)?)?
        .ok_or("original envelope absent")?;
    check!(eq; value.at(), "2026-05-15");
    check!(eq; value.payload().get(), r#"{"value":1}"#);
    Ok(())
}

#[test]
fn new_timestamp_route_refuses_missing_invalid_duplicate_and_oversized_time() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for raw in [
        r#"{"key":"record","payload":{}}"#.to_owned(),
        r#"{"key":"record","at":3,"payload":{}}"#.to_owned(),
        r#"{"key":"record","at":"a","at":"b","payload":{}}"#.to_owned(),
        format!(
            r#"{{"key":"record","at":"{}","payload":{{}}}}"#,
            "x".repeat(129)
        ),
    ] {
        store
            .journal
            .insert(Store::journal_key(PHASE, KEY), raw.as_bytes())?;
        check!(store
            .snapshot()
            .journal_entry_raw_bounded(PHASE, KEY, JournalReadBudget::new(4096)?)
            .is_err());
    }
    store.journal.insert(
        Store::journal_key(PHASE, KEY),
        br#"{"key":"record","payload":{}}"#,
    )?;
    check!(store
        .snapshot()
        .journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(4096)?)?
        .is_some());
    Ok(())
}

#[test]
fn timestamped_raw_envelope_uses_existing_chunk_reassembly() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = json!({"value": "x".repeat(MAX_JOURNAL_VALUE_BYTES + 1)});
    store.journal_done(PHASE, KEY, &payload)?;
    let snapshot = store.snapshot();
    let budget = JournalReadBudget::new(MAX_JOURNAL_VALUE_BYTES * 4)?;
    let raw = snapshot
        .journal_payload_raw_bounded(PHASE, KEY, budget)?
        .ok_or("chunked payload absent")?;
    let entry = snapshot
        .journal_entry_raw_bounded(PHASE, KEY, budget)?
        .ok_or("chunked envelope absent")?;
    check!(!entry.at().is_empty());
    check!(eq; entry.payload().get(), raw.get());
    check!(snapshot
        .journal_entry_raw_bounded(PHASE, KEY, JournalReadBudget::new(64)?)
        .is_err());
    Ok(())
}
