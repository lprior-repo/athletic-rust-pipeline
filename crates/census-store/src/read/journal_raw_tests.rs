use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};

use crate::journal::{chunk_key, chunk_value, manifest_value, JOURNAL_CHUNK_RAW};
use crate::{JournalReadBudget, Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type RawResult = StoreResult<Option<Box<RawValue>>>;
const PHASE: &str = "raw_journal_contract";
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

fn text(result: RawResult) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(result?.map(|raw| raw.get().to_owned()))
}

fn exact(store: &Store, encoded_bytes: usize, expected: &str) -> TestResult {
    let grant = JournalReadBudget::new(encoded_bytes)?;
    check!(eq; text(store.journal_payload_raw_bounded(PHASE, KEY, grant))?, Some(expected.to_owned()));
    check!(eq; text(store.snapshot().journal_payload_raw_bounded(PHASE, KEY, grant))?, Some(expected.to_owned()));
    Ok(())
}

fn descriptor(result: RawResult, expected: &str) -> TestResult {
    match result {
        Err(StoreError::JournalDescriptor { phase, key, detail }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            check!(eq; detail, expected);
            Ok(())
        }
        found => Err(format!("expected descriptor {expected}, found {found:?}").into()),
    }
}

fn duplicate(result: RawResult, field: &str) -> TestResult {
    match result {
        Err(StoreError::Decode { source, .. }) => {
            check!(eq; source.classify(), serde_json::error::Category::Data);
            check!(source
                .to_string()
                .contains(&format!("duplicate legacy envelope field {field}")));
            Ok(())
        }
        found => Err(format!("expected duplicate {field}, found {found:?}").into()),
    }
}

fn limit(result: RawResult, needed: usize, max: usize) -> TestResult {
    match result {
        Err(StoreError::JournalReadLimit {
            phase,
            key,
            what,
            needed: found,
            max: ceiling,
        }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            check!(eq; what, "encoded envelope");
            check!(eq; found, needed);
            check!(eq; ceiling, max);
            Ok(())
        }
        found => Err(format!("expected encoded limit, found {found:?}").into()),
    }
}

#[test]
fn raw_missing_is_none_and_null_is_a_present_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let grant = JournalReadBudget::new(1)?;
    check!(eq; text(store.journal_payload_raw_bounded(PHASE, KEY, grant))?, None);
    check!(eq; text(store.snapshot().journal_payload_raw_bounded(PHASE, KEY, grant))?, None);
    let raw = envelope("null");
    root(&store, raw.as_bytes())?;
    exact(&store, raw.len(), "null")
}

#[test]
fn raw_inline_retains_spacing_escapes_order_and_numeric_spelling() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = r#"{ "z":1e+02, "a":"\u0061", "nested": [true, null, "🦀"] }"#;
    let raw = envelope(payload);
    root(&store, raw.as_bytes())?;
    exact(&store, raw.len(), payload)?;
    check!(eq; store.journal_payload(PHASE, KEY)?, Some(serde_json::from_str::<Value>(payload)?));
    let lower = raw.len().checked_sub(1).ok_or("fixture underflow")?;
    limit(
        store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(lower)?),
        raw.len(),
        lower,
    )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownPayload {
    known: Option<String>,
}

#[test]
fn raw_payload_keeps_escaped_and_null_first_duplicates_for_typed_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for payload in [
        r#"{"known":"first","known":"second"}"#,
        r#"{"known":null,"known":"second"}"#,
        r#"{"known":null,"\u006bnown":"second"}"#,
    ] {
        let raw = envelope(payload);
        for write in [root, chunked] {
            write(&store, raw.as_bytes())?;
            exact(&store, raw.len(), payload)?;
            let retained = store
                .journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(raw.len())?)?
                .ok_or("payload missing")?;
            match serde_json::from_str::<KnownPayload>(retained.get()) {
                Err(error) => check!(error.to_string().contains("duplicate field `known`")),
                Ok(value) => return Err(format!("duplicate accepted: {:?}", value.known).into()),
            }
            check!(eq; store.journal_payload(PHASE, KEY)?, Some(json!({"known":"second"})));
        }
    }
    Ok(())
}

#[test]
fn raw_chunked_retains_exact_bytes_across_boundaries_and_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let payload = format!(
        "{{ \"blob\":\"{}\", \"x\":null, \"x\":1e0 }}",
        "z".repeat(JOURNAL_CHUNK_RAW)
    );
    let raw = envelope(&payload);
    {
        let store = Store::open(directory.path())?;
        chunked(&store, raw.as_bytes())?;
        exact(&store, raw.len(), &payload)?;
        store.flush()?;
    }
    let store = Store::open(directory.path())?;
    exact(&store, raw.len(), &payload)
}

#[test]
fn raw_snapshot_keeps_retained_chunk_bytes_when_latest_root_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = r#"{"known":null,"known":"second"}"#;
    let raw = envelope(payload);
    chunked(&store, raw.as_bytes())?;
    let view = store.snapshot();
    root(&store, envelope("false").as_bytes())?;
    let grant = JournalReadBudget::new(raw.len())?;
    check!(eq; text(view.journal_payload_raw_bounded(PHASE, KEY, grant))?, Some(payload.to_owned()));
    check!(eq; text(store.journal_payload_raw_bounded(PHASE, KEY, grant))?, Some("false".to_owned()));
    Ok(())
}

#[test]
fn raw_declared_budget_precedes_missing_chunk_or_allocation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    root(&store, &manifest_value(KEY, 1, 128)?)?;
    limit(
        store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(127)?),
        128,
        127,
    )?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(128)?),
        "chunk 0 is missing",
    )
}

#[test]
fn raw_refuses_outer_duplicates_without_changing_legacy_value_semantics() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for (raw, field) in [
        (
            r#"{"key":"retained","payload":null,"payload":true}"#,
            "payload",
        ),
        (
            r#"{"key":"retained","payload":null,"\u0070ayload":true}"#,
            "payload",
        ),
        (
            r#"{"key":null,"\u006bey":"retained","payload":true}"#,
            "key",
        ),
        (
            r#"{"key":"retained","at":null,"at":"old","payload":true}"#,
            "at",
        ),
    ] {
        for write in [root, chunked] {
            write(&store, raw.as_bytes())?;
            duplicate(
                store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(raw.len())?),
                field,
            )?;
            check!(eq; store.journal_payload(PHASE, KEY)?, Some(json!(true)));
        }
    }
    Ok(())
}

#[test]
fn raw_refuses_duplicate_manifest_and_chunk_fields() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let body = envelope("true");
    chunked(&store, body.as_bytes())?;
    let manifest = format!("{{\"type\":\"chunked-manifest\",\"version\":1,\"key\":\"{KEY}\",\"chunks\":null,\"chunks\":1,\"total_bytes\":{}}}", body.len());
    root(&store, manifest.as_bytes())?;
    let grant = JournalReadBudget::new(body.len())?;
    duplicate(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "chunks",
    )?;
    check!(eq; store.journal_payload(PHASE, KEY)?, Some(json!(true)));
    root(&store, &manifest_value(KEY, 1, body.len())?)?;
    let raw = String::from_utf8(chunk_value(KEY, 0, 1, body.as_bytes())?)?;
    let raw = raw.replacen("{", "{\"data\":null,", 1);
    chunk(&store, 0, raw.as_bytes())?;
    duplicate(store.journal_payload_raw_bounded(PHASE, KEY, grant), "data")?;
    check!(eq; store.journal_payload(PHASE, KEY)?, Some(json!(true)));
    Ok(())
}

#[test]
fn raw_corrupt_chunks_and_foreign_envelopes_keep_typed_refusals() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let grant = JournalReadBudget::new(1024)?;
    root(&store, &manifest_value(KEY, 1, 2)?)?;
    chunk(
        &store,
        0,
        br#"{"type":"chunk","version":1,"key":"retained","index":0,"total":1,"data":"%%%="}"#,
    )?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "chunk 0 data is not valid base64",
    )?;
    chunk(&store, 0, &chunk_value(KEY, 0, 1, b"x")?)?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "chunk 0 decoded length does not match its manifest",
    )?;
    root(&store, br#"{"key":"foreign","payload":true}"#)?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "entry key does not match its lookup",
    )?;
    root(&store, br#"{"key":"retained"}"#)?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "entry has no payload",
    )?;
    root(
        &store,
        br#"{"type":"journal-alias","version":2,"key":"retained","payload":true}"#,
    )?;
    descriptor(
        store.journal_payload_raw_bounded(PHASE, KEY, grant),
        "root is neither an inline entry nor a chunked-manifest",
    )
}

#[test]
fn raw_reconstruction_rejects_trailing_json_and_preserves_stored_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let raw = format!("{}{{}}", envelope("true"));
    chunked(&store, raw.as_bytes())?;
    let before = store.journal.get(Store::journal_key(PHASE, KEY))?;
    match store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(raw.len())?) {
        Err(StoreError::Decode { key, source }) => {
            check!(eq; key, format!("legacy journal reassembled entry for {PHASE}/{KEY}"));
            check!(eq; source.classify(), serde_json::error::Category::Syntax);
        }
        found => return Err(format!("trailing JSON returned {found:?}").into()),
    }
    check!(eq; store.journal.get(Store::journal_key(PHASE, KEY))?, before);
    Ok(())
}

#[test]
fn raw_unknown_header_refuses_without_changing_permissive_value_reader() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let nested = json!({"list": [{"deep": "x".repeat(8192)}]});
    let raw = format!("{{\"key\":\"{KEY}\",\"unknown\":{nested},\"payload\":true}}");
    for write in [root, chunked] {
        write(&store, raw.as_bytes())?;
        match store.journal_payload_raw_bounded(PHASE, KEY, JournalReadBudget::new(raw.len())?) {
            Err(StoreError::Decode { source, .. }) => {
                check!(source.to_string().contains("unknown field `unknown`"));
            }
            found => return Err(format!("unknown header accepted: {found:?}").into()),
        }
        check!(eq; store.journal_payload(PHASE, KEY)?, Some(json!(true)));
    }
    Ok(())
}

#[test]
fn raw_physical_root_and_chunk_refuse_before_json_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let length = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("fixture overflow")?;
    let invalid = vec![b'x'; length];
    let grant = JournalReadBudget::new(1024)?;
    root(&store, &invalid)?;
    for what in ["physical root", "physical chunk"] {
        match store.journal_payload_raw_bounded(PHASE, KEY, grant) {
            Err(StoreError::JournalReadLimit {
                what: found,
                needed,
                max,
                ..
            }) => {
                check!(eq; found, what);
                check!(eq; needed, length);
                check!(eq; max, MAX_JOURNAL_VALUE_BYTES);
            }
            found => return Err(format!("physical ceiling returned {found:?}").into()),
        }
        root(&store, &manifest_value(KEY, 1, 128)?)?;
        chunk(&store, 0, &invalid)?;
    }
    Ok(())
}
