use serde_json::{json, Value};

use crate::journal::{
    chunk_key, chunk_value, encode_chunk, manifest_value, JOURNAL_CHUNK_RAW,
    OBJECT_RECORD_BYTES_LIMIT,
};
use crate::{JournalReadBudget, Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PHASE: &str = "legacy_bounded_contract";
const KEY: &str = "retained";

fn entry(payload: Value) -> Value {
    json!({"key": KEY, "at": "2026-10-11", "payload": payload})
}

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

fn chunked(store: &Store, payload: &Value) -> Result<usize, Box<dyn std::error::Error>> {
    let raw = serde_json::to_vec(&entry(payload.clone()))?;
    let pieces: Vec<_> = raw.chunks(JOURNAL_CHUNK_RAW).collect();
    root(store, &manifest_value(KEY, pieces.len(), raw.len())?)?;
    for (index, piece) in pieces.iter().enumerate() {
        chunk(store, index, &chunk_value(KEY, index, pieces.len(), piece)?)?;
    }
    Ok(raw.len())
}

fn limit(
    result: StoreResult<Option<Value>>,
    expected_what: &str,
    expected_needed: usize,
    expected_max: usize,
) -> TestResult {
    match result {
        Err(StoreError::JournalReadLimit {
            phase,
            key,
            what,
            needed,
            max,
        }) => {
            check!(eq; phase, PHASE);
            check!(eq; key, KEY);
            check!(eq; what, expected_what);
            check!(eq; needed, expected_needed);
            check!(eq; max, expected_max);
            Ok(())
        }
        found => Err(format!("expected {expected_what} limit, found {found:?}").into()),
    }
}

fn descriptor(result: StoreResult<Option<Value>>, expected: &str) -> TestResult {
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

#[test]
fn bounded_budget_accepts_only_nonzero_supported_encoded_ceiling() -> TestResult {
    for bytes in [1, OBJECT_RECORD_BYTES_LIMIT] {
        check!(eq; JournalReadBudget::new(bytes)?.max_encoded_bytes(), bytes);
    }
    for bytes in [0, OBJECT_RECORD_BYTES_LIMIT + 1, usize::MAX] {
        let expected =
            format!("journal read budget must be in 1..={OBJECT_RECORD_BYTES_LIMIT} encoded bytes");
        match JournalReadBudget::new(bytes) {
            Err(StoreError::Refused { detail }) => check!(eq; detail, expected),
            found => return Err(format!("invalid budget returned {found:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn bounded_missing_entry_is_none_even_with_minimum_budget() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let budget = JournalReadBudget::new(1)?;
    check!(eq; store.journal_payload_bounded(PHASE, KEY, budget)?, None);
    check!(eq; store.snapshot().journal_payload_bounded(PHASE, KEY, budget)?, None);
    Ok(())
}

#[test]
fn bounded_inline_charges_exact_encoded_envelope_and_preserves_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = json!({"unicode": "🦀", "nested": [null, true, {"key": "unrelated"}]});
    let mut raw = serde_json::to_vec(&entry(payload.clone()))?;
    raw.extend_from_slice(b" \n");
    root(&store, &raw)?;
    let grant = JournalReadBudget::new(raw.len())?;
    check!(eq; store.journal_payload_bounded(PHASE, KEY, grant)?, Some(payload.clone()));
    check!(eq; store.snapshot().journal_payload_bounded(PHASE, KEY, grant)?, Some(payload));
    let lower = raw.len().checked_sub(1).ok_or("fixture underflow")?;
    limit(
        store.journal_payload_bounded(PHASE, KEY, JournalReadBudget::new(lower)?),
        "encoded envelope",
        raw.len(),
        lower,
    )
}

#[test]
fn bounded_chunked_charges_declared_bytes_before_reading_missing_chunks() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    root(&store, &manifest_value(KEY, 1, 128)?)?;
    limit(
        store.journal_payload_bounded(PHASE, KEY, JournalReadBudget::new(127)?),
        "encoded envelope",
        128,
        127,
    )?;
    descriptor(
        store.journal_payload_bounded(PHASE, KEY, JournalReadBudget::new(128)?),
        "chunk 0 is missing",
    )
}

#[test]
fn bounded_chunked_preserves_historical_padding_lengths_and_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    {
        let store = Store::open(directory.path())?;
        for bytes in [0, 1, 2, 3, JOURNAL_CHUNK_RAW, JOURNAL_CHUNK_RAW * 2] {
            let payload = json!({"blob": "z".repeat(bytes)});
            let encoded = chunked(&store, &payload)?;
            let grant = JournalReadBudget::new(encoded)?;
            check!(eq; store.journal_payload_bounded(PHASE, KEY, grant)?, Some(payload));
        }
        store.flush()?;
    }
    let store = Store::open(directory.path())?;
    let payload = json!({"blob": "z".repeat(JOURNAL_CHUNK_RAW * 2)});
    check!(eq; store.journal_payload(PHASE, KEY)?, Some(payload));
    Ok(())
}

#[test]
fn bounded_snapshot_keeps_original_chunked_bytes_when_latest_keys_change() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let original = json!({"blob": "a".repeat(1_100_000)});
    let replacement = json!({"blob": "b".repeat(1_100_000)});
    store.journal_done(PHASE, KEY, &original)?;
    let view = store.snapshot();
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, KEY, &replacement)?;
    batch.journal_done(PHASE, "other", &json!({"changed": true}))?;
    batch.commit()?;
    let budget = JournalReadBudget::new(2_000_000)?;
    check!(eq; view.journal_payload_bounded(PHASE, KEY, budget)?, Some(original));
    check!(eq; view.journal_payload_bounded(PHASE, "other", budget)?, None);
    check!(eq; store.journal_payload_bounded(PHASE, KEY, budget)?, Some(replacement));
    Ok(())
}

#[test]
fn bounded_chunks_reject_physical_and_codec_ceilings_with_exact_typed_context() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let body = serde_json::to_vec(&entry(json!("original")))?;
    root(&store, &manifest_value(KEY, 1, body.len())?)?;
    let mut physical = chunk_value(KEY, 0, 1, &body)?;
    physical.resize(MAX_JOURNAL_VALUE_BYTES + 1, b' ');
    chunk(&store, 0, &physical)?;
    limit(
        store.journal_payload(PHASE, KEY),
        "physical chunk",
        physical.len(),
        MAX_JOURNAL_VALUE_BYTES,
    )?;
    let encoded_max = JOURNAL_CHUNK_RAW.div_ceil(3) * 4;
    let mut value: Value = serde_json::from_slice(&chunk_value(KEY, 0, 1, &body)?)?;
    value["data"] = json!("A".repeat(encoded_max + 4));
    chunk(&store, 0, &serde_json::to_vec(&value)?)?;
    limit(
        store.journal_payload(PHASE, KEY),
        "chunk base64",
        encoded_max + 4,
        encoded_max,
    )?;
    value["data"] = json!(encode_chunk(&vec![b'x'; JOURNAL_CHUNK_RAW + 1]));
    chunk(&store, 0, &serde_json::to_vec(&value)?)?;
    limit(
        store.journal_payload(PHASE, KEY),
        "chunk decoded",
        JOURNAL_CHUNK_RAW + 1,
        JOURNAL_CHUNK_RAW,
    )
}

#[test]
fn bounded_root_refuses_alias_fallback_and_missing_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    root(
        &store,
        &serde_json::to_vec(
            &json!({"type":"journal-alias", "version":2, "key":KEY, "payload":true}),
        )?,
    )?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "root is neither an inline entry nor a chunked-manifest",
    )?;
    root(&store, &serde_json::to_vec(&json!({"key":KEY}))?)?;
    descriptor(store.journal_payload(PHASE, KEY), "entry has no payload")
}

#[test]
fn bounded_manifest_refuses_malformed_unsigned_lengths_and_zero_bytes() -> TestResult {
    for (field, value, detail) in [
        (
            "total_bytes",
            json!(-1),
            "manifest has no unsigned total_bytes",
        ),
        (
            "total_bytes",
            json!("64"),
            "manifest has no unsigned total_bytes",
        ),
        (
            "total_bytes",
            Value::Null,
            "manifest has no unsigned total_bytes",
        ),
        (
            "total_bytes",
            json!(0),
            "manifest has no encoded envelope bytes",
        ),
        ("chunks", json!(-1), "manifest has no unsigned chunks"),
        ("chunks", json!("1"), "manifest has no unsigned chunks"),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut manifest: Value = serde_json::from_slice(&manifest_value(KEY, 1, 64)?)?;
        manifest[field] = value;
        root(&store, &serde_json::to_vec(&manifest)?)?;
        descriptor(store.journal_payload(PHASE, KEY), detail)?;
    }
    Ok(())
}

#[test]
fn bounded_chunk_refuses_invalid_base64_and_missing_data() -> TestResult {
    for (data, detail) in [
        (Some(json!("%%%=")), "chunk 0 data is not valid base64"),
        (Some(json!("YQ")), "chunk 0 data is not valid base64"),
        (Some(Value::Null), "chunk 0 has no base64 data"),
        (None, "chunk 0 has no base64 data"),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        root(&store, &manifest_value(KEY, 1, 2)?)?;
        let mut value: Value = serde_json::from_slice(&chunk_value(KEY, 0, 1, b"{}")?)?;
        if let Some(data) = data {
            value["data"] = data;
        } else {
            value
                .as_object_mut()
                .ok_or("fixture not object")?
                .remove("data");
        }
        chunk(&store, 0, &serde_json::to_vec(&value)?)?;
        descriptor(store.journal_payload(PHASE, KEY), detail)?;
    }
    Ok(())
}

#[test]
fn bounded_reconstruction_requires_complete_json_not_a_valid_prefix() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut body = serde_json::to_vec(&entry(json!(true)))?;
    body.extend_from_slice(b"{}");
    root(&store, &manifest_value(KEY, 1, body.len())?)?;
    chunk(&store, 0, &chunk_value(KEY, 0, 1, &body)?)?;
    match store.journal_payload(PHASE, KEY) {
        Err(StoreError::Decode { key, source }) => {
            check!(eq; key, format!("legacy journal reassembled entry for {PHASE}/{KEY}"));
            check!(eq; source.classify(), serde_json::error::Category::Syntax);
            Ok(())
        }
        found => Err(format!("trailing JSON returned {found:?}").into()),
    }
}

#[test]
fn bounded_physical_root_accepts_exact_ceiling_and_refuses_one_extra_byte() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = json!({"retained": true});
    let mut raw = serde_json::to_vec(&entry(payload.clone()))?;
    raw.resize(MAX_JOURNAL_VALUE_BYTES, b' ');
    root(&store, &raw)?;
    check!(eq; store.journal_payload(PHASE, KEY)?, Some(payload));
    raw.push(b' ');
    root(&store, &raw)?;
    limit(
        store.journal_payload(PHASE, KEY),
        "physical root",
        raw.len(),
        MAX_JOURNAL_VALUE_BYTES,
    )?;
    let retained = store
        .journal
        .get(Store::journal_key(PHASE, KEY))?
        .ok_or("root disappeared")?;
    check!(eq; retained.as_ref(), raw.as_slice());
    Ok(())
}

#[test]
fn bounded_manifest_rejects_huge_chunk_count_before_reserve_or_iteration() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut value: Value = serde_json::from_slice(&manifest_value(KEY, 1, 64)?)?;
    value["chunks"] = json!(u64::MAX);
    root(&store, &serde_json::to_vec(&value)?)?;
    let expected = if usize::BITS == 64 {
        "manifest chunk count does not cover its byte length"
    } else {
        "manifest chunks cannot be represented"
    };
    descriptor(store.journal_payload(PHASE, KEY), expected)
}

#[test]
fn bounded_reconstruction_rejects_a_wrapped_alias_or_versioned_entry() -> TestResult {
    for fields in [
        json!({"type":"journal-alias","version":2}),
        json!({"version":2}),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut value = entry(json!(true));
        value
            .as_object_mut()
            .ok_or("entry object")?
            .extend(fields.as_object().ok_or("fields object")?.clone());
        let body = serde_json::to_vec(&value)?;
        root(&store, &manifest_value(KEY, 1, body.len())?)?;
        chunk(&store, 0, &chunk_value(KEY, 0, 1, &body)?)?;
        descriptor(
            store.journal_payload(PHASE, KEY),
            "entry contains an unsupported type or version",
        )?;
    }
    Ok(())
}

#[test]
fn bounded_inline_rejects_an_unrecognized_top_level_version() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut value = entry(json!({"version":2}));
    value["version"] = json!(2);
    root(&store, &serde_json::to_vec(&value)?)?;
    descriptor(
        store.journal_payload(PHASE, KEY),
        "entry contains an unsupported type or version",
    )
}
