use super::super::{Entry, RecordedPayload, RetainedJournalBundle};
use crate::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_store::read::{JournalNamespace, ReadBudget};
use census_store::{Store, StoreError, StoreSnapshot};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.store_identity()?;
    Ok((directory, store))
}

fn present(phase: &str, key: &str, value: Value) -> Entry {
    Entry {
        phase: phase.into(),
        key: key.into(),
        payload: RecordedPayload::Present {
            value,
            minimum_successful_read_budget: 4_194_304,
        },
    }
}

fn oracle(view: &StoreSnapshot<'_>, entries: Vec<Entry>) -> TestResult<Value> {
    let mut payload_bytes = 0_u64;
    let mut maximum = 0_u64;
    for entry in &entries {
        let size = u64::try_from(serde_json::to_vec(&entry.payload)?.len())?;
        payload_bytes = payload_bytes
            .checked_add(size)
            .ok_or("oracle payload overflow")?;
        maximum = maximum.max(size);
    }
    let count = u64::try_from(entries.len())?;
    let bundle = RetainedJournalBundle {
        version: 1,
        store_identity: view.store_identity()?,
        snapshot_sequence: view.sequence(),
        entries,
    };
    let bytes = serde_json::to_vec(&bundle)?;
    let report = serde_json::to_value(RetainedJournalBundle::measure_present(view)?)?;
    check!(eq; report.as_object().ok_or("report object")?.len(), 10);
    check!(eq; report.get("scope"), Some(&json!("present_phase_superset")));
    check!(eq; report.get("store_identity"), Some(&serde_json::to_value(view.store_identity()?)?));
    check!(eq; report.get("snapshot_sequence"), Some(&json!(view.sequence())));
    check!(eq; report.get("present_entries"), Some(&json!(count)));
    check!(eq; report.get("recorded_payload_bytes"), Some(&json!(payload_bytes)));
    check!(eq; report.get("maximum_recorded_payload_bytes"), Some(&json!(maximum)));
    check!(eq; report.get("canonical_bundle_bytes"), Some(&json!(u64::try_from(bytes.len())?)));
    check!(eq; report.get("canonical_digest"), Some(&json!(format!("{:x}", Sha256::digest(&bytes)))));
    check!(eq; report.get("assumed_read_grant_bytes"), Some(&json!(4_194_304)));
    let mut phases = Vec::new();
    for phase in [OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE] {
        let inventory = view.for_each_journal_root_bounded(
            JournalNamespace::new(phase, "row/")?,
            ReadBudget {
                rows: 100,
                bytes: 8_388_608,
                record_bytes: 1_048_576,
            },
            |_| Ok(()),
        )?;
        phases.push(json!({
            "phase": phase, "logical_roots": inventory.logical_roots(),
            "physical_rows": inventory.fingerprint().physical_rows(),
            "physical_encoded_bytes": inventory.fingerprint().encoded_bytes(),
        }));
    }
    check!(eq; report.get("phases"), Some(&json!(phases)));
    Ok(report)
}

#[test]
fn empty_phases_match_existing_empty_bundle_without_changing_store_state() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = store.snapshot();
    let report = oracle(&before, Vec::new())?;
    check!(eq; report.get("recorded_payload_bytes"), Some(&json!(0)));
    check!(eq; report.get("maximum_recorded_payload_bytes"), Some(&json!(0)));
    check!(eq; store.snapshot().sequence(), before.sequence());
    check!(eq; store.snapshot().evidence_generation(), before.evidence_generation());
    check!(eq; store.snapshot().derived_generation(), before.derived_generation());
    check!(eq; store.snapshot().store_identity()?, before.store_identity()?);
    Ok(())
}

#[test]
fn one_null_and_two_escaped_entries_match_independent_existing_bundle_bytes() -> TestResult {
    let (_directory, store) = fixture()?;
    store.journal_done(OWNED_CAPTURE_PHASE, "row/z-null", &Value::Null)?;
    let held = store.snapshot();
    let first = oracle(
        &held,
        vec![present(OWNED_CAPTURE_PHASE, "row/z-null", Value::Null)],
    )?;
    let key = "row/a\"\\é";
    let value = json!({"slash":"\\", "line":"\n", "quote":"\"", "unicode":"é🦀", "nested":{"z":null,"a":[false,1]}});
    store.journal_done(OWNED_MEET_PHASE, key, &value)?;
    let repeated = oracle(
        &held,
        vec![present(OWNED_CAPTURE_PHASE, "row/z-null", Value::Null)],
    )?;
    check!(eq; first, repeated);
    let current = store.snapshot();
    let second = oracle(
        &current,
        vec![
            present(OWNED_CAPTURE_PHASE, "row/z-null", Value::Null),
            present(OWNED_MEET_PHASE, key, value),
        ],
    )?;
    check!(eq; second.get("present_entries"), Some(&json!(2)));
    check!(first.get("canonical_digest") != second.get("canonical_digest"));
    check!(eq; store.snapshot().sequence(), current.sequence());
    Ok(())
}

#[test]
fn entry_order_uses_phase_then_key_and_unrelated_phase_is_excluded() -> TestResult {
    let (_directory, store) = fixture()?;
    store.journal_done(OWNED_MEET_PHASE, "row/a", &json!({"revision": 3}))?;
    store.journal_done(OWNED_CAPTURE_PHASE, "row/z", &json!({"revision": 2}))?;
    store.journal_done(OWNED_CAPTURE_PHASE, "row/a", &json!({"revision": 1}))?;
    store.journal_done(
        "milesplit_owned_capture_v10",
        "row/a",
        &json!({"foreign": true}),
    )?;
    let before = store.snapshot();
    let report = oracle(
        &before,
        vec![
            present(OWNED_CAPTURE_PHASE, "row/a", json!({"revision": 1})),
            present(OWNED_CAPTURE_PHASE, "row/z", json!({"revision": 2})),
            present(OWNED_MEET_PHASE, "row/a", json!({"revision": 3})),
        ],
    )?;
    check!(eq; report.get("present_entries"), Some(&json!(3)));
    check!(eq; store.snapshot().sequence(), before.sequence());
    Ok(())
}

#[test]
fn physically_chunked_payload_is_one_present_entry_in_the_canonical_oracle() -> TestResult {
    let (_directory, store) = fixture()?;
    let value = json!({"body": "x".repeat(1_100_000)});
    store.journal_done(OWNED_CAPTURE_PHASE, "row/chunked", &value)?;
    let report = oracle(
        &store.snapshot(),
        vec![present(OWNED_CAPTURE_PHASE, "row/chunked", value)],
    )?;
    check!(eq; report.get("present_entries"), Some(&json!(1)));
    let first = report
        .get("phases")
        .and_then(Value::as_array)
        .and_then(|phases| phases.first())
        .ok_or("capture phase")?;
    check!(eq; first.get("physical_rows"), Some(&json!(4)));
    check!(eq; first.get("logical_roots"), Some(&json!(1)));
    Ok(())
}

#[test]
fn changed_payload_changes_new_report_while_held_snapshot_stays_exact() -> TestResult {
    let (_directory, store) = fixture()?;
    store.journal_done(OWNED_MEET_PHASE, "row/a", &json!({"value": "old"}))?;
    let held = store.snapshot();
    let first = oracle(
        &held,
        vec![present(OWNED_MEET_PHASE, "row/a", json!({"value": "old"}))],
    )?;
    store.journal_done(OWNED_MEET_PHASE, "row/a", &json!({"value": "new"}))?;
    let repeated = oracle(
        &held,
        vec![present(OWNED_MEET_PHASE, "row/a", json!({"value": "old"}))],
    )?;
    check!(eq; first, repeated);
    let current = store.snapshot();
    let changed = oracle(
        &current,
        vec![present(OWNED_MEET_PHASE, "row/a", json!({"value": "new"}))],
    )?;
    check!(first.get("canonical_digest") != changed.get("canonical_digest"));
    check!(eq; store.snapshot().sequence(), current.sequence());
    Ok(())
}

#[test]
fn exact_512_byte_utf8_key_succeeds_and_next_byte_refuses_without_mutation() -> TestResult {
    let (_directory, store) = fixture()?;
    let valid = format!("row/{}", "é".repeat(254));
    check!(eq; valid.len(), 512);
    store.journal_done(OWNED_CAPTURE_PHASE, &valid, &Value::Null)?;
    oracle(
        &store.snapshot(),
        vec![present(OWNED_CAPTURE_PHASE, &valid, Value::Null)],
    )?;
    let invalid = format!("{valid}x");
    check!(eq; invalid.len(), 513);
    store.journal_done(OWNED_CAPTURE_PHASE, &invalid, &Value::Null)?;
    let before = store.snapshot();
    check!(matches!(RetainedJournalBundle::measure_present(&before),
        Err(StoreError::Refused { detail }) if detail == "unsupported retained source journal key"));
    check!(eq; store.snapshot().sequence(), before.sequence());
    Ok(())
}

#[test]
fn actual_payload_exceeding_fixed_read_grant_refuses_instead_of_partial_report() -> TestResult {
    let (_directory, store) = fixture()?;
    store.journal_done(OWNED_CAPTURE_PHASE, "row/a", &Value::Null)?;
    store.journal_done(OWNED_CAPTURE_PHASE, "row/z", &"x".repeat(4_194_304))?;
    let before = store.snapshot();
    check!(matches!(RetainedJournalBundle::measure_present(&before),
        Err(StoreError::JournalReadLimit { phase, key, what, needed, max: 4_194_304 })
            if phase == OWNED_CAPTURE_PHASE && key == "row/z"
                && what == "encoded envelope" && needed > 4_194_304));
    check!(eq; store.snapshot().sequence(), before.sequence());
    Ok(())
}

#[test]
fn canonical_counter_accepts_exact_32_gib_and_refuses_next_byte_and_overflow() -> TestResult {
    check!(eq; super::count::MAX_CANONICAL_BYTES, 34_359_738_368);
    check!(eq; super::count::charge(0, 0)?, 0);
    check!(eq; super::count::charge(34_359_738_367, 1)?, 34_359_738_368);
    check!(eq; super::count::charge(34_359_738_368, 0)?, 34_359_738_368);
    for (current, reason) in [
        (34_359_738_368, "byte budget exceeded"),
        (u64::MAX, "counter overflow"),
    ] {
        let error = super::count::charge(current, 1)
            .err()
            .ok_or("counter accepted excess")?;
        check!(eq; error.kind(), std::io::ErrorKind::Other);
        check!(eq; error.to_string(), reason);
    }
    Ok(())
}

mod progress;
