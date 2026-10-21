use serde::Deserialize;
use serde_json::value::RawValue;

use crate::journal::{chunk_key, chunk_value, manifest_value, JOURNAL_CHUNK_RAW};
use crate::read::with_retained_journal_entry_raw_bounded;
use crate::{JournalReadBudget, Store, StoreError};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PHASE: &str = "borrowed_journal_regression";
const KEY: &str = "retained";

fn root(store: &Store, raw: &[u8]) -> TestResult {
    store.journal.insert(Store::journal_key(PHASE, KEY), raw)?;
    Ok(())
}

fn chunk(store: &Store, index: usize, raw: &[u8]) -> TestResult {
    store
        .journal
        .insert(chunk_key(&Store::journal_key(PHASE, KEY), index)?, raw)?;
    Ok(())
}

fn chunked(store: &Store, raw: &[u8]) -> TestResult {
    let pieces: Vec<_> = raw.chunks(JOURNAL_CHUNK_RAW).collect();
    root(store, &manifest_value(KEY, pieces.len(), raw.len())?)?;
    for (index, piece) in pieces.iter().enumerate() {
        chunk(store, index, &chunk_value(KEY, index, pieces.len(), piece)?)?;
    }
    Ok(())
}

fn envelope(payload: &str) -> String {
    format!("{{\"key\":\"{KEY}\",\"at\":\"historical\",\"payload\": {payload} }}\n")
}

#[derive(Debug, Deserialize)]
struct Projection {
    known: Option<String>,
    count: Option<u64>,
}

fn project(raw: &RawValue) -> crate::StoreResult<Projection> {
    serde_json::from_str(raw.get()).map_err(|source| StoreError::Decode {
        key: format!("borrowed journal payload for {PHASE}/{KEY}"),
        source,
    })
}

fn descriptor(result: crate::StoreResult<Option<Projection>>) -> TestResult {
    match result {
        Err(StoreError::JournalDescriptor { phase, key, .. }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            Ok(())
        }
        found => Err(format!("expected JournalDescriptor, found {found:?}").into()),
    }
}

fn decode_error(result: crate::StoreResult<Option<Projection>>) -> TestResult {
    match result {
        Err(StoreError::Decode { source, .. }) => {
            check!(eq; source.classify(), serde_json::error::Category::Data);
            Ok(())
        }
        found => Err(format!("expected Decode error, found {found:?}").into()),
    }
}

fn limit_error(result: crate::StoreResult<Option<Projection>>) -> TestResult {
    match result {
        Err(StoreError::JournalReadLimit {
            phase,
            key,
            needed,
            max,
            ..
        }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            check!(needed > max);
            Ok(())
        }
        found => Err(format!("expected limit error, found {found:?}").into()),
    }
}

#[test]
fn borrowed_inline_projects_exact_metadata() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = r#"{"known":"alpha","count":42}"#;
    let raw = envelope(payload);
    root(&store, raw.as_bytes())?;

    let grant = JournalReadBudget::new(raw.len())?;
    let result =
        with_retained_journal_entry_raw_bounded(&store.snapshot(), PHASE, KEY, grant, project)?;
    let projection = result.ok_or("expected Some(projection)")?;
    check!(eq; projection.known, Some("alpha".to_string()));
    check!(eq; projection.count, Some(42));

    let physical = store
        .journal
        .get(Store::journal_key(PHASE, KEY))?
        .ok_or("root disappeared")?;
    check!(eq; physical.as_ref(), raw.as_bytes());
    Ok(())
}

#[test]
fn borrowed_chunked_projects_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let payload = format!(
        "{{\"known\":\"large\",\"count\":99,\"blob\":\"{}\"}}",
        "z".repeat(JOURNAL_CHUNK_RAW)
    );
    let raw = envelope(&payload);
    {
        let store = Store::open(directory.path())?;
        chunked(&store, raw.as_bytes())?;
        store.flush()?;
    }
    let store = Store::open(directory.path())?;
    let grant = JournalReadBudget::new(raw.len())?;
    let result =
        with_retained_journal_entry_raw_bounded(&store.snapshot(), PHASE, KEY, grant, project)?;
    let projection = result.ok_or("expected Some(projection)")?;
    check!(eq; projection.known, Some("large".to_string()));
    check!(eq; projection.count, Some(99));
    Ok(())
}

#[test]
fn borrowed_refuses_malformed_envelopes_and_budget_violations() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let read = |budget| -> crate::StoreResult<Option<Projection>> {
        with_retained_journal_entry_raw_bounded(
            &store.snapshot(),
            PHASE,
            KEY,
            JournalReadBudget::new(budget)?,
            project,
        )
    };
    for raw in [
        r#"{"key":"retained","payload":{"known":"a"}}"#,
        r#"{"key":"wrong","at":"t","payload":{"known":"a"}}"#,
        r#"{"key":"retained","at":"t"}"#,
        r#"{"key":"retained","at":"t","type":"inline","payload":null}"#,
    ] {
        root(&store, raw.as_bytes())?;
        descriptor(read(1024))?;
    }
    for raw in [
        r#"{"key":"retained","at":42,"payload":null}"#,
        r#"{"key":"retained","at":"t","payload":null,"payload":true}"#,
        r#"{"key":"retained","at":"t","payload":null,"unknown_field":true}"#,
    ] {
        root(&store, raw.as_bytes())?;
        decode_error(read(1024))?;
    }
    root(&store, envelope(r#"{"known":"a","known":"b"}"#).as_bytes())?;
    decode_error(read(1024))?;
    let oversized_at = format!(
        "{{\"key\":\"retained\",\"at\":\"{}\",\"payload\":null}}",
        "x".repeat(129),
    );
    root(&store, oversized_at.as_bytes())?;
    limit_error(read(2048))?;
    let declared = JOURNAL_CHUNK_RAW
        .checked_mul(2)
        .ok_or("chunk total overflow")?;
    root(&store, &manifest_value(KEY, 2, declared)?)?;
    let body = vec![0u8; JOURNAL_CHUNK_RAW];
    chunk(&store, 0, &chunk_value(KEY, 0, 2, &body)?)?;
    descriptor(read(declared))?;
    chunk(&store, 1, &chunk_value(KEY, 1, 2, &body)?)?;
    let mut corrupt: serde_json::Value = serde_json::from_slice(&chunk_value(KEY, 0, 2, &body)?)?;
    *corrupt.get_mut("data").ok_or("chunk data")? = serde_json::json!("!!!");
    chunk(&store, 0, &serde_json::to_vec(&corrupt)?)?;
    descriptor(read(declared))?;
    root(&store, &manifest_value(KEY, 1, JOURNAL_CHUNK_RAW)?)?;
    for (field, value) in [("version", 999), ("index", 5), ("total", 2)] {
        let mut invalid: serde_json::Value =
            serde_json::from_slice(&chunk_value(KEY, 0, 1, &body)?)?;
        *invalid.get_mut(field).ok_or("chunk field")? = serde_json::json!(value);
        chunk(&store, 0, &serde_json::to_vec(&invalid)?)?;
        descriptor(read(JOURNAL_CHUNK_RAW))?;
    }
    let raw = envelope(r#"{"known":"tiny"}"#);
    root(&store, raw.as_bytes())?;
    limit_error(read(raw.len().checked_sub(1).ok_or("budget underflow")?))?;
    root(&store, &manifest_value(KEY, 1, JOURNAL_CHUNK_RAW)?)?;
    limit_error(read(64))?;
    Ok(())
}
