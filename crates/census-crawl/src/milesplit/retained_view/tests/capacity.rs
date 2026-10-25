use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PHASE: &str = crate::milesplit::OWNED_CAPTURE_PHASE;

fn refused<T>(result: StoreResult<T>, expected: &str) -> TestResult {
    match result.err().ok_or("unexpected success")? {
        StoreError::Refused { detail } => check!(eq; detail, expected),
        other => return Err(format!("unexpected error: {other:?}").into()),
    }
    Ok(())
}

fn present(value: Value, grant: usize) -> RecordedPayload {
    RecordedPayload::Present {
        value,
        minimum_successful_read_budget: grant,
    }
}

#[test]
fn numeric_recording_limits_are_inclusive_checked_and_finite() -> TestResult {
    check!(eq; MAX_ENTRIES, 4_000_000);
    check!(eq; MAX_BYTES, 8_589_934_592);
    check!(eq; admit_recording_bytes(MAX_BYTES - 1, 1)?, MAX_BYTES);
    check!(matches!(
        admit_recording_bytes(MAX_BYTES, 1),
        Err(StoreError::BudgetExceeded {
            what: "retained journal recording bytes",
            needed: 8_589_934_593,
            available: 8_589_934_592
        })
    ));
    check!(matches!(
        admit_recording_bytes(usize::MAX, 1),
        Err(StoreError::CounterOverflow)
    ));
    Ok(())
}

#[test]
fn actual_reconciliation_deduplicates_minimum_grant_and_refuses_changed_value() -> TestResult {
    let mut original = present(Value::Null, 4096);
    reconcile(&mut original, &present(Value::Null, 2048))?;
    check!(eq; original, present(Value::Null, 2048));
    reconcile(&mut original, &present(Value::Null, 8192))?;
    check!(eq; original, present(Value::Null, 2048));
    for changed in [RecordedPayload::Absent, present(Value::Bool(true), 2048)] {
        refused(
            reconcile(&mut original, &changed),
            "pinned retained journal changed during recording",
        )?;
        check!(eq; original, present(Value::Null, 2048));
    }
    refused(
        reconcile(&mut RecordedPayload::Absent, &original),
        "pinned retained journal changed during recording",
    )?;
    reconcile(&mut RecordedPayload::Absent, &RecordedPayload::Absent)?;
    Ok(())
}

#[test]
fn genuine_reads_deduplicate_and_cold_journal_preserves_null_absence_and_grants() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.store_identity()?;
    store.journal_done(PHASE, "synthetic/null", &Value::Null)?;
    let snapshot = store.snapshot();
    let recorder = RecordingRetainedJournal::new(&snapshot);
    check!(eq; recorder.journal_payload_bounded(PHASE, "synthetic/null", JournalReadBudget::new(4096)?)?, Some(Value::Null));
    let charged = recorder.bytes.get();
    check!(eq; recorder.journal_payload_bounded(PHASE, "synthetic/null", JournalReadBudget::new(2048)?)?, Some(Value::Null));
    check!(eq; recorder.bytes.get(), charged);
    check!(eq; recorder.journal_payload_bounded(PHASE, "synthetic/absent", JournalReadBudget::new(2048)?)?, None);
    let bundle = recorder.finish()?;
    check!(eq; bundle.entries.len(), 2);
    check!(eq; bundle.store_identity.as_str(), snapshot.store_identity()?.as_str());
    check!(eq; bundle.snapshot_sequence, snapshot.sequence());
    let bytes = bundle.encode()?;
    let digest = crate::net::cache::content_digest(&bytes);
    let identity = bundle.store_identity.as_str().to_owned();
    let sequence = bundle.snapshot_sequence;
    drop(snapshot);
    drop(store);
    let replay = FrozenRetainedJournal::from_descriptor(&digest, &identity, sequence, &bytes)?;
    check!(eq; replay.journal_payload_bounded(PHASE, "synthetic/null", JournalReadBudget::new(2048)?)?, Some(Value::Null));
    check!(eq; replay.journal_payload_bounded(PHASE, "synthetic/absent", JournalReadBudget::new(1)?)?, None);
    match replay.journal_payload_bounded(PHASE, "synthetic/null", JournalReadBudget::new(2047)?) {
        Err(StoreError::JournalReadLimit {
            phase,
            key,
            what,
            needed,
            max,
        }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, "synthetic/null");
            check!(eq; what, "recorded successful read budget");
            check!(eq; needed, 2048);
            check!(eq; max, 2047);
        }
        _ => return Err("remembered successful grant was not enforced".into()),
    }
    refused(
        replay.journal_payload_bounded(PHASE, "unlisted", JournalReadBudget::new(2048)?),
        "requested source object is outside frozen journal inventory",
    )?;
    Ok(())
}

#[test]
fn synthetic_near_limit_seed_refuses_actual_read_without_mutation_and_poisons_finish() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.store_identity()?;
    let snapshot = store.snapshot();
    let recorder = RecordingRetainedJournal::new(&snapshot);
    recorder.bytes.set(MAX_BYTES);
    let charge = serde_json::to_vec(&RecordedPayload::Absent)?.len() as u64;
    match recorder.journal_payload_bounded(PHASE, "missing", JournalReadBudget::new(1)?) {
        Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        }) => {
            check!(eq; what, "retained journal recording bytes");
            check!(eq; needed, MAX_BYTES as u64 + charge);
            check!(eq; available, MAX_BYTES as u64);
        }
        _ => return Err("actual recording byte refusal missing".into()),
    }
    check!(recorder.entries.borrow().is_empty());
    check!(eq; recorder.bytes.get(), MAX_BYTES);
    refused(
        recorder.finish(),
        "retained journal recording cannot freeze an unrecorded failed read",
    )?;
    Ok(())
}

#[test]
fn genuine_read_grant_failure_cannot_freeze_an_absent_substitute() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = census_store::Store::open(directory.path())?;
    store.store_identity()?;
    store.journal_done(PHASE, "large", &serde_json::json!({"value":"large"}))?;
    let snapshot = store.snapshot();
    let expected =
        read_limit(snapshot.journal_payload_bounded(PHASE, "large", JournalReadBudget::new(1)?))?;
    let recorder = RecordingRetainedJournal::new(&snapshot);
    check!(eq; read_limit(recorder.journal_payload_bounded(
        PHASE, "large", JournalReadBudget::new(1)?))?, expected);
    check!(recorder.entries.borrow().is_empty());
    check!(eq; recorder.bytes.get(), 0);
    refused(
        recorder.finish(),
        "retained journal recording cannot freeze an unrecorded failed read",
    )?;
    Ok(())
}

fn read_limit(result: StoreResult<Option<Value>>) -> TestResult<usize> {
    match result.err().ok_or("unexpected read success")? {
        StoreError::JournalReadLimit {
            phase,
            key,
            what,
            needed,
            max,
        } => {
            check!(eq; phase, PHASE);
            check!(eq; key, "large");
            check!(eq; what, "encoded envelope");
            check!(eq; max, 1);
            check!(needed > max);
            Ok(needed)
        }
        other => Err(format!("unexpected read failure: {other:?}").into()),
    }
}
