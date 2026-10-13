use serde_json::{json, Value};

use crate::journal::{chunk_key, chunk_value, manifest_value, OBJECT_RECORD_BYTES_LIMIT};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PHASE: &str = "legacy_read_regression";
const KEY: &str = "retained";

fn entry(key: &str) -> Value {
    json!({"key": key, "at": "2026-10-11", "payload": {"original": true}})
}

fn write_root(store: &Store, raw: &[u8]) -> TestResult {
    store.journal.insert(Store::journal_key(PHASE, KEY), raw)?;
    Ok(())
}

fn write_chunk(store: &Store, index: usize, value: &Value) -> TestResult {
    let physical = chunk_key(&Store::journal_key(PHASE, KEY), index)?;
    store.journal.insert(physical, serde_json::to_vec(value)?)?;
    Ok(())
}

fn one_chunk(store: &Store, body: &[u8]) -> TestResult {
    write_root(store, &manifest_value(KEY, 1, body.len())?)?;
    let value: Value = serde_json::from_slice(&chunk_value(KEY, 0, 1, body)?)?;
    write_chunk(store, 0, &value)
}

fn descriptor(result: StoreResult<Option<Value>>, detail: &str) -> TestResult {
    match result {
        Err(StoreError::JournalDescriptor {
            phase,
            key,
            detail: found,
        }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            check!(eq; found, detail);
            Ok(())
        }
        found => Err(format!("expected descriptor {detail:?}, found {found:?}").into()),
    }
}

fn limit_display(
    result: StoreResult<Option<Value>>,
    what: &str,
    needed: usize,
    max: usize,
) -> TestResult {
    let expected = format!(
        "journal read {what} for {PHASE}/{KEY} needs {needed} bytes, past the {max} ceiling"
    );
    match result {
        Err(found) => {
            check!(eq; found.to_string(), expected);
            Ok(())
        }
        found => Err(format!("expected {expected:?}, found {found:?}").into()),
    }
}

#[test]
fn legacy_inline_rejects_a_foreign_entry_key() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    write_root(&store, &serde_json::to_vec(&entry("foreign"))?)?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "entry key does not match its lookup",
    )
}

#[test]
fn legacy_manifest_rejects_a_foreign_key_before_chunk_read() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let body = serde_json::to_vec(&entry(KEY))?;
    one_chunk(&store, &body)?;
    write_root(&store, &manifest_value("foreign", 1, body.len())?)?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "manifest key does not match its lookup",
    )
}

#[test]
fn legacy_manifest_rejects_an_unsupported_version() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let body = serde_json::to_vec(&entry(KEY))?;
    one_chunk(&store, &body)?;
    let mut value: Value = serde_json::from_slice(&manifest_value(KEY, 1, body.len())?)?;
    value["version"] = json!(2);
    write_root(&store, &serde_json::to_vec(&value)?)?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "manifest is not version 1",
    )
}

#[test]
fn legacy_chunks_reject_swapped_foreign_ownership_and_ordinals() -> TestResult {
    for (field, value, detail) in [
        ("type", json!("foreign"), "chunk 0 is not a chunk envelope"),
        ("version", json!(2), "chunk 0 is not version 1"),
        (
            "key",
            json!("foreign"),
            "chunk 0 key does not match its lookup",
        ),
        ("index", json!(1), "chunk 0 ordinal does not match its key"),
        (
            "total",
            json!(2),
            "chunk 0 total does not match its manifest",
        ),
    ] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let body = serde_json::to_vec(&entry(KEY))?;
        one_chunk(&store, &body)?;
        let mut chunk: Value = serde_json::from_slice(&chunk_value(KEY, 0, 1, &body)?)?;
        chunk[field] = value;
        write_chunk(&store, 0, &chunk)?;
        descriptor(store.journal_payload(PHASE, KEY), detail)?;
    }
    Ok(())
}

#[test]
fn legacy_chunked_entry_rejects_a_foreign_reconstructed_key() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    one_chunk(&store, &serde_json::to_vec(&entry("foreign"))?)?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "entry key does not match its lookup",
    )
}

#[test]
fn legacy_manifest_rejects_zero_or_inexact_chunk_counts() -> TestResult {
    for chunks in [0, 2] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let body = serde_json::to_vec(&entry(KEY))?;
        one_chunk(&store, &body)?;
        write_root(&store, &manifest_value(KEY, chunks, body.len())?)?;
        descriptor(
            store.journal_payload(PHASE, KEY),
            "manifest chunk count does not cover its byte length",
        )?;
    }
    Ok(())
}

#[test]
fn legacy_manifest_rejects_truncated_or_extra_decoded_bytes() -> TestResult {
    for delta in [-1_isize, 1] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let body = serde_json::to_vec(&entry(KEY))?;
        one_chunk(&store, &body)?;
        let total = body
            .len()
            .checked_add_signed(delta)
            .ok_or("fixture overflow")?;
        write_root(&store, &manifest_value(KEY, 1, total)?)?;
        descriptor(
            store.journal_payload(PHASE, KEY),
            "chunk 0 decoded length does not match its manifest",
        )?;
    }
    Ok(())
}

#[test]
fn legacy_default_rejects_over_ceiling_declarations_without_huge_allocation() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let body = serde_json::to_vec(&entry(KEY))?;
    one_chunk(&store, &body)?;
    let declared = OBJECT_RECORD_BYTES_LIMIT
        .checked_add(1)
        .ok_or("fixture overflow")?;
    write_root(&store, &manifest_value(KEY, 1, declared)?)?;
    limit_display(
        store.journal_payload(PHASE, KEY),
        "encoded envelope",
        declared,
        OBJECT_RECORD_BYTES_LIMIT,
    )
}

#[test]
fn legacy_default_rejects_over_ceiling_physical_roots_read_only() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut raw = serde_json::to_vec(&entry(KEY))?;
    let oversized = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("fixture overflow")?;
    raw.resize(oversized, b' ');
    write_root(&store, &raw)?;
    limit_display(
        store.journal_payload(PHASE, KEY),
        "physical root",
        oversized,
        MAX_JOURNAL_VALUE_BYTES,
    )?;
    let retained = store
        .journal
        .get(Store::journal_key(PHASE, KEY))?
        .ok_or("root disappeared")?;
    check!(eq; retained.as_ref(), raw.as_slice());
    Ok(())
}
