use serde_json::{json, Value};

use super::{run_pass, stage_context_object, ContextRecordSink, ObjectPlan, Pass};
use crate::journal::{
    object_chunk_key, object_digest, object_index_key, object_manifest_key, JOURNAL_OBJECT_VERSION,
    OBJECT_ALIAS_TYPE, OBJECT_CHUNK_BYTES, OBJECT_STATE_READY,
};
use crate::{Store, StoreError, StoreResult};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod boundaries;
mod failures;

fn envelope(index: usize, text: &str) -> Value {
    json!({"key": format!("row-{index}"), "at": "2026-10-11T00:00:00Z", "payload": text})
}

fn emit(sink: &mut ContextRecordSink<'_>, records: &[Value]) -> StoreResult<()> {
    records.iter().try_for_each(|value| sink.record(value))
}

fn plan(records: &[Value]) -> StoreResult<ObjectPlan> {
    run_pass(Pass::Measure, &|sink| emit(sink, records))
}

fn write_row(store: &Store, key: &[u8], value: &[u8]) -> TestResult {
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, value);
    batch.commit()?;
    Ok(())
}

fn alias(store: &Store, object: &str) -> TestResult {
    let value = json!({
        "type": OBJECT_ALIAS_TYPE,
        "version": JOURNAL_OBJECT_VERSION,
        "state": OBJECT_STATE_READY,
        "object": object,
    });
    write_row(
        store,
        &Store::journal_key("context-test", "ready"),
        &serde_json::to_vec(&value)?,
    )
}

fn refused(result: StoreResult<super::StagedContextObject>, message: &str) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) => check!(detail.contains(message), "{detail}"),
        other => return Err(format!("expected refusal {message:?}, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn staged_context_has_exact_jsonl_digest_and_legacy_reader_ranges() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records: Vec<_> = (0..513)
        .map(|index| envelope(index, &"é".repeat(1100)))
        .collect();
    let expected: Vec<u8> = records.iter().try_fold(Vec::new(), |mut bytes, value| {
        bytes.extend(serde_json::to_vec(value)?);
        bytes.push(b'\n');
        Ok::<_, serde_json::Error>(bytes)
    })?;
    let before = store.snapshot();
    let staged = stage_context_object(&store, |sink| emit(sink, &records))?;
    let after = store.snapshot();
    check!(eq; staged.object_id(), object_digest(&expected));
    check!(eq; staged.total_bytes(), expected.len());
    check!(eq; staged.record_count(), 513);
    check!(eq; staged.chunk_count(), expected.len().div_ceil(OBJECT_CHUNK_BYTES));
    check!(eq; after.evidence_generation(), before.evidence_generation());
    check!(eq; after.derived_generation(), before.derived_generation());
    check!(eq; store.journal.len()?, 6);
    check!(store.journal.prefix(b"public_identity").next().is_none());
    check!(store
        .journal
        .get(Store::journal_key("context-test", "ready"))?
        .is_none());
    alias(&store, staged.object_id())?;
    let object = store
        .journal_object("context-test", "ready")?
        .ok_or("missing test alias")?;
    let count = object.records().try_fold(0_usize, |offset, record| {
        let record = record?;
        let expected_value = records.get(record.sequence()).ok_or("record sequence")?;
        check!(eq; record.offset(), offset);
        check!(eq; record.payload(), expected_value.get("payload").ok_or("payload")?);
        check!(eq; record.length(), serde_json::to_vec(expected_value)?.len() + 1);
        Ok::<_, Box<dyn std::error::Error>>(offset + record.length())
    })?;
    check!(eq; count, expected.len());
    Ok(())
}

#[test]
fn empty_context_has_one_empty_chunk_and_no_index_page() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_context_object(&store, |_| Ok(()))?;
    check!(eq; staged.object_id(), object_digest(b""));
    check!(eq; staged.total_bytes(), 0);
    check!(eq; staged.record_count(), 0);
    check!(eq; staged.chunk_count(), 1);
    check!(eq; store.journal.len()?, 2);
    alias(&store, staged.object_id())?;
    let object = store
        .journal_object("context-test", "ready")?
        .ok_or("empty context alias")?;
    check!(eq; object.records().count(), 0);
    Ok(())
}

#[test]
fn identical_context_replay_is_a_durable_no_op_after_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let records = [envelope(0, "original")];
    let staged = {
        let store = Store::open(dir.path())?;
        stage_context_object(&store, |sink| emit(sink, &records))?
    };
    let store = Store::open(dir.path())?;
    let before = store.snapshot().sequence();
    let replay = stage_context_object(&store, |sink| emit(sink, &records))?;
    check!(eq; replay, staged);
    check!(eq; store.snapshot().sequence(), before);
    check!(eq; store.journal.len()?, 3);
    Ok(())
}

#[test]
fn manifest_is_absent_until_all_callback_records_are_staged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = [envelope(0, &"x".repeat(600_000)), envelope(1, "tail")];
    let expected = plan(&records)?;
    let key = object_manifest_key(expected.staged.object_id())?;
    let pass = std::cell::Cell::new(0);
    stage_context_object(&store, |sink| {
        pass.set(pass.get() + 1);
        records.iter().try_for_each(|record| {
            sink.record(record)?;
            if store
                .journal
                .get(&key)
                .map_err(|source| StoreError::Read { source })?
                .is_some()
            {
                return Err(StoreError::Refused {
                    detail: "premature ready manifest".to_string(),
                });
            }
            Ok(())
        })
    })?;
    check!(eq; pass.get(), 2);
    check!(store.journal.get(key)?.is_some());
    Ok(())
}

mod charge;
