mod raw_generation;

use serde_json::{json, Value};

use crate::journal::{
    encode_chunk, object_chunk_key, object_digest, object_index_key, object_manifest_key,
    JOURNAL_OBJECT_VERSION, OBJECT_ALIAS_TYPE, OBJECT_BYTES_LIMIT, OBJECT_CHUNKS_LIMIT,
    OBJECT_CHUNK_BYTES, OBJECT_CHUNK_TYPE, OBJECT_CODEC_JSONL, OBJECT_INDEX_PAGE_ENTRIES,
    OBJECT_INDEX_TYPE, OBJECT_MANIFEST_TYPE, OBJECT_RECORDS_LIMIT, OBJECT_STATE_READY,
};
use crate::read::{JournalObject, JournalRecord};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PHASE: &str = "journal_object_read";
const KEY: &str = "settled";
const AT: &str = "2026-10-11T00:00:00Z";

struct ObjectPlan {
    id: String,
    payload: Vec<u8>,
    chunks: Vec<Vec<u8>>,
    ranges: Vec<(usize, usize)>,
}

fn record(key: &str, payload: Value) -> Value {
    json!({"key": key, "at": AT, "payload": payload})
}

fn entries(specs: &[(&str, usize)]) -> Vec<Value> {
    specs
        .iter()
        .map(|(key, rows)| record(key, json!({"rows": rows})))
        .collect()
}

fn large(key: &str) -> Value {
    record(key, json!({"blob": "x".repeat(600 * 1024)}))
}

fn plan(records: &[Value]) -> TestResult<ObjectPlan> {
    let mut payload = Vec::new();
    let mut ranges = Vec::new();
    for record in records {
        let bytes = serde_json::to_vec(record)?;
        let length = bytes.len().checked_add(1).ok_or("record range overflow")?;
        let offset = payload.len();
        payload.extend_from_slice(&bytes);
        payload.push(b'\n');
        ranges.push((offset, length));
    }
    let id = object_digest(&payload);
    let chunks = if payload.is_empty() {
        vec![Vec::new()]
    } else {
        payload
            .chunks(OBJECT_CHUNK_BYTES)
            .map(|chunk| chunk.to_vec())
            .collect()
    };
    Ok(ObjectPlan {
        id,
        payload,
        chunks,
        ranges,
    })
}

fn chunk_record_forged(object: &str, index: usize, length: usize, bytes: &[u8]) -> Value {
    json!({
        "type": OBJECT_CHUNK_TYPE,
        "version": JOURNAL_OBJECT_VERSION,
        "object": object,
        "index": index,
        "length": length,
        "digest": object_digest(bytes),
        "data": encode_chunk(bytes),
    })
}

fn chunk_record(object: &str, index: usize, bytes: &[u8]) -> Value {
    chunk_record_forged(object, index, bytes.len(), bytes)
}

fn index_record(object: &str, page: usize, ranges: &[(usize, usize)], first: usize) -> Value {
    let entries: Vec<Value> = ranges
        .iter()
        .enumerate()
        .map(|(position, (offset, length))| {
            json!({"sequence": first + position, "offset": offset, "length": length})
        })
        .collect();
    json!({
        "type": OBJECT_INDEX_TYPE,
        "version": JOURNAL_OBJECT_VERSION,
        "object": object,
        "page": page,
        "entries": entries,
    })
}

fn manifest_record(object: &str, staged: &ObjectPlan) -> Value {
    let digests: Vec<String> = staged
        .chunks
        .iter()
        .map(|chunk| object_digest(chunk))
        .collect();
    json!({
        "type": OBJECT_MANIFEST_TYPE,
        "version": JOURNAL_OBJECT_VERSION,
        "object": object,
        "codec": OBJECT_CODEC_JSONL,
        "total_bytes": staged.payload.len(),
        "chunks": staged.chunks.len(),
        "records": staged.ranges.len(),
        "index_pages": staged.ranges.len().div_ceil(OBJECT_INDEX_PAGE_ENTRIES),
        "chunk_digests": digests,
    })
}

fn alias_record(object: &str) -> Value {
    json!({
        "type": OBJECT_ALIAS_TYPE,
        "version": JOURNAL_OBJECT_VERSION,
        "state": OBJECT_STATE_READY,
        "object": object,
    })
}

fn pairs(staged: &ObjectPlan, object: &str) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut out = Vec::new();
    for (index, chunk) in staged.chunks.iter().enumerate() {
        out.push((
            object_chunk_key(object, index)?,
            serde_json::to_vec(&chunk_record(object, index, chunk))?,
        ));
    }
    let pages = staged.ranges.len().div_ceil(OBJECT_INDEX_PAGE_ENTRIES);
    for page in 0..pages {
        let start = page * OBJECT_INDEX_PAGE_ENTRIES;
        let end = std::cmp::min(start + OBJECT_INDEX_PAGE_ENTRIES, staged.ranges.len());
        let slice = staged.ranges.get(start..end).ok_or("index page slice")?;
        out.push((
            object_index_key(object, page)?,
            serde_json::to_vec(&index_record(object, page, slice, start))?,
        ));
    }
    out.push((
        object_manifest_key(object)?,
        serde_json::to_vec(&manifest_record(object, staged))?,
    ));
    Ok(out)
}

fn write(store: &Store, pairs: &[(Vec<u8>, Vec<u8>)]) -> TestResult {
    let mut batch = store.db.batch();
    for (key, value) in pairs {
        batch.insert(&store.journal, key, value);
    }
    batch.commit()?;
    Ok(())
}

fn alias_pair(object: &str) -> TestResult<(Vec<u8>, Vec<u8>)> {
    Ok((
        Store::journal_key(PHASE, KEY),
        serde_json::to_vec(&alias_record(object))?,
    ))
}

fn stage_under(store: &Store, staged: &ObjectPlan, object: &str) -> TestResult {
    let mut all = pairs(staged, object)?;
    all.push(alias_pair(object)?);
    write(store, &all)
}

fn stage(store: &Store, staged: &ObjectPlan) -> TestResult {
    let object = staged.id.clone();
    stage_under(store, staged, &object)
}

fn stage_records(store: &Store, records: &[Value]) -> TestResult<ObjectPlan> {
    let staged = plan(records)?;
    stage(store, &staged)?;
    Ok(staged)
}

fn stage_without(store: &Store, staged: &ObjectPlan, excluded: &[u8]) -> TestResult {
    let mut all = pairs(staged, &staged.id)?;
    all.retain(|(key, _)| key.as_slice() != excluded);
    all.push(alias_pair(&staged.id)?);
    write(store, &all)
}

fn read(store: &Store) -> StoreResult<Option<JournalObject<'_>>> {
    store.journal_object(PHASE, KEY)
}

fn stream(object: &JournalObject<'_>) -> TestResult<Vec<JournalRecord>> {
    Ok(object.records().collect::<Result<Vec<_>, _>>()?)
}

#[test]
fn journal_object_streams_typed_records_from_one_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records = vec![
        record("first", json!({"rows": 1})),
        large("second"),
        record("third", json!({"rows": 3})),
    ];
    let staged = stage_records(&store, &records)?;
    check!(staged.chunks.len() > 1, "chunks={}", staged.chunks.len());
    let object = read(&store)?.ok_or("journal object")?;
    check!(eq; object.object_id(), staged.id.as_str());
    check!(eq; object.total_bytes(), staged.payload.len());
    check!(eq; object.record_count(), records.len());
    check!(eq; object.chunk_count(), staged.chunks.len());
    let mut cursor = object.records();
    let first = cursor.next().ok_or("first record")??;
    check!(eq; first.sequence(), 0);
    check!(eq; first.offset(), staged.ranges[0].0);
    check!(eq; first.length(), staged.ranges[0].1);
    check!(eq; first.entry().key.as_str(), "first");
    check!(eq; first.payload(), &json!({"rows": 1}));
    let second = cursor.next().ok_or("second record")??;
    check!(eq; second.sequence(), 1);
    let crosses = staged.ranges[1].0 < OBJECT_CHUNK_BYTES
        && staged.ranges[1].0 + staged.ranges[1].1 > OBJECT_CHUNK_BYTES;
    check!(crosses, "the second record stays inside one chunk");
    check!(eq; second.entry().key.as_str(), "second");
    let third = cursor.next().ok_or("third record")??;
    check!(eq; third.sequence(), 2);
    check!(eq; third.entry().key.as_str(), "third");
    check!(cursor.next().is_none(), "the stream holds extra records");
    Ok(())
}

#[test]
fn journal_object_streams_an_empty_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &[])?;
    check!(eq; staged.payload.len(), 0);
    check!(eq; staged.chunks.len(), 1);
    let object = read(&store)?.ok_or("journal object")?;
    check!(eq; object.total_bytes(), 0);
    check!(eq; object.chunk_count(), 1);
    check!(eq; object.record_count(), 0);
    check!(eq; object.records().count(), 0);
    Ok(())
}

#[test]
fn journal_object_stream_yields_records_positionally() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut records = entries(&[("alpha", 1), ("beta", 2), ("gamma", 3)]);
    records[1] = json!({"key": "beta", "at": AT, "payload": {"rows": 2}, "extra": true});
    let staged = stage_records(&store, &records)?;
    check!(eq; staged.ranges.len(), 3);
    let object = read(&store)?.ok_or("journal object")?;
    let mut cursor = object.records();
    check!(eq; cursor.next().ok_or("first")??.entry().key.as_str(), "alpha");
    let refused = cursor.next().ok_or("second")?;
    check!(matches!(refused, Err(StoreError::Decode { .. })));
    check!(
        cursor.next().is_none(),
        "the stream continued after a refusal"
    );
    Ok(())
}

#[test]
fn journal_object_refuses_a_legacy_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(PHASE, KEY, &json!({"rows": 1}))?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalDescriptor { .. })
    ));
    let oversized = "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    store.journal_done(PHASE, "chunked", &oversized)?;
    check!(matches!(
        store.journal_object(PHASE, "chunked"),
        Err(StoreError::JournalDescriptor { .. })
    ));
    check!(matches!(store.journal_object(PHASE, "absent"), Ok(None)));
    Ok(())
}

#[test]
fn journal_object_refuses_a_descriptor_that_is_not_ready() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &entries(&[("alpha", 1)]))?;
    let mut pending = alias_record(&staged.id);
    pending["state"] = json!("pending");
    write(
        &store,
        &[(
            Store::journal_key(PHASE, KEY),
            serde_json::to_vec(&pending)?,
        )],
    )?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalNotReady { state, .. }) if state == "pending"
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_missing_manifest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("alpha", 1)]))?;
    write(&store, &[alias_pair(&staged.id)?])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalManifestMissing { object, .. }) if object == staged.id
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_missing_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&[large("first"), record("second", json!({"rows": 2}))])?;
    check!(staged.chunks.len() > 1);
    let excluded = object_chunk_key(&staged.id, 1)?;
    stage_without(&store, &staged, &excluded)?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkMissing { index: 1, .. })
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_an_extra_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &[large("first")])?;
    let declared = staged.chunks.len();
    let extra = chunk_record(&staged.id, declared, b"extra generation");
    let pair = (
        object_chunk_key(&staged.id, declared)?,
        serde_json::to_vec(&extra)?,
    );
    write(&store, &[pair])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkExtra { index, declared: bound, .. })
            if index == declared && bound == declared
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_stale_chunk_digest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let stale = vec![b'z'; staged.chunks[0].len()];
    let pair = (
        object_chunk_key(&staged.id, 0)?,
        serde_json::to_vec(&chunk_record(&staged.id, 0, &stale))?,
    );
    write(&store, &[pair])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkDigest { index: 0, declared, computed, .. })
            if declared == object_digest(&staged.chunks[0]) && computed == object_digest(&stale)
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_truncated_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let expected = staged.chunks[0].len();
    let mut short = staged.chunks[0].clone();
    short.truncate(expected - 1);
    let pair = (
        object_chunk_key(&staged.id, 0)?,
        serde_json::to_vec(&chunk_record_forged(&staged.id, 0, expected, &short))?,
    );
    write(&store, &[pair])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkLength { index: 0, declared, observed, .. })
            if declared == expected && observed == expected - 1
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_chunk_length_past_its_descriptor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let expected = staged.chunks[0].len();
    let pair = (
        object_chunk_key(&staged.id, 0)?,
        serde_json::to_vec(&chunk_record_forged(&staged.id, 0, expected + 1, b""))?,
    );
    write(&store, &[pair])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkLength { index: 0, declared, observed, .. })
            if declared == expected && observed == expected + 1
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_missing_and_extra_index_pages() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records: Vec<Value> = (0..513)
        .map(|index| record(&format!("unit-{index}"), json!({"rows": index})))
        .collect();
    let staged = plan(&records)?;
    check!(eq; staged.ranges.len(), 513);
    check!(staged.ranges.len().div_ceil(OBJECT_INDEX_PAGE_ENTRIES) == 2);
    let excluded = object_index_key(&staged.id, 1)?;
    stage_without(&store, &staged, &excluded)?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalIndexMissing { page: 1, .. })
    ));
    let extra = index_record(&staged.id, 2, &[], 513);
    let pair = (
        object_index_key(&staged.id, 2)?,
        serde_json::to_vec(&extra)?,
    );
    write(&store, &[pair])?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalIndexExtra {
            page: 2,
            declared: 2,
            ..
        })
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_final_digest_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("alpha", 1), ("beta", 2)]))?;
    let forged = object_digest(b"another generation");
    check!(forged != staged.id, "the forged id equals the plan id");
    stage_under(&store, &staged, &forged)?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalObjectDigest { declared, computed, .. })
            if declared == forged && computed == staged.id
    ));
    Ok(())
}

#[test]
fn journal_object_validates_counts_before_reading_any_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("alpha", 1)]))?;
    let base = manifest_record(&staged.id, &staged);
    let mut bytes_case = base.clone();
    bytes_case["total_bytes"] = json!(OBJECT_BYTES_LIMIT + 1);
    let pair = (
        object_manifest_key(&staged.id)?,
        serde_json::to_vec(&bytes_case)?,
    );
    write(&store, &[pair, alias_pair(&staged.id)?])?;
    check!(matches!(
        read(&store),
        Err(StoreError::BudgetExceeded { what, .. }) if what == "journal object bytes"
    ));
    let mut chunks_case = base.clone();
    chunks_case["chunks"] = json!(OBJECT_CHUNKS_LIMIT + 1);
    let pair = (
        object_manifest_key(&staged.id)?,
        serde_json::to_vec(&chunks_case)?,
    );
    write(&store, &[pair, alias_pair(&staged.id)?])?;
    check!(matches!(
        read(&store),
        Err(StoreError::BudgetExceeded { what, .. }) if what == "journal object chunks"
    ));
    let mut records_case = base.clone();
    records_case["records"] = json!(OBJECT_RECORDS_LIMIT + 1);
    let pair = (
        object_manifest_key(&staged.id)?,
        serde_json::to_vec(&records_case)?,
    );
    write(&store, &[pair, alias_pair(&staged.id)?])?;
    check!(matches!(
        read(&store),
        Err(StoreError::BudgetExceeded { what, .. }) if what == "journal object records"
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_an_oversized_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("alpha", 1)]))?;
    let mut oversized = index_record(&staged.id, 0, &[(0, 1)], 0);
    oversized["entries"] = json!([{
        "sequence": 0,
        "offset": 0,
        "length": MAX_JOURNAL_VALUE_BYTES * 1024,
    }]);
    let mut all = pairs(&staged, &staged.id)?;
    all.push((
        object_index_key(&staged.id, 0)?,
        serde_json::to_vec(&oversized)?,
    ));
    all.push(alias_pair(&staged.id)?);
    write(&store, &all)?;
    check!(matches!(read(&store), Err(StoreError::RowTooLarge { .. })));
    Ok(())
}

#[test]
fn journal_object_pins_one_generation_across_an_alias_overwrite() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let captured = store.snapshot();
    let pinned = read(&store)?.ok_or("pinned object")?;
    let second = stage_records(&store, &entries(&[("alpha", 9), ("beta", 8)]))?;
    check!(first.id != second.id, "the rewrite reused the object id");
    check!(eq; pinned.object_id(), first.id.as_str());
    let streamed = stream(&pinned)?;
    check!(eq; streamed.len(), 2);
    check!(eq; streamed[0].entry().key.as_str(), "alpha");
    check!(eq; streamed[0].payload(), &json!({"rows": 1}));
    check!(eq; streamed[1].payload(), &json!({"rows": 2}));
    let through_capture = captured.journal_object(PHASE, KEY)?.ok_or("captured")?;
    check!(eq; through_capture.object_id(), first.id.as_str());
    let recaptured = stream(&through_capture)?;
    check!(eq; recaptured[0].payload(), &json!({"rows": 1}));
    let current = store
        .snapshot()
        .journal_object(PHASE, KEY)?
        .ok_or("current")?;
    check!(eq; current.object_id(), second.id.as_str());
    let restreamed = stream(&current)?;
    check!(eq; restreamed[0].payload(), &json!({"rows": 9}));
    Ok(())
}

#[test]
fn journal_object_refuses_a_manifest_interleaved_from_another_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let second = stage_records(&store, &entries(&[("alpha", 9), ("beta", 8)]))?;
    check!(first.id != second.id);
    check!(eq; first.chunks[0].len(), second.chunks[0].len());
    let current = store
        .snapshot()
        .journal_object(PHASE, KEY)?
        .ok_or("current")?;
    check!(eq; current.object_id(), second.id.as_str());
    let swapped = manifest_record(&first.id, &second);
    write(
        &store,
        &[
            (
                object_manifest_key(&first.id)?,
                serde_json::to_vec(&swapped)?,
            ),
            alias_pair(&first.id)?,
        ],
    )?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkDigest { index: 0, declared, computed, .. })
            if declared == object_digest(&second.chunks[0])
                && computed == object_digest(&first.chunks[0])
    ));
    Ok(())
}

#[test]
fn journal_object_refuses_a_chunk_interleaved_from_another_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let captured = store.snapshot();
    let second = stage_records(&store, &entries(&[("alpha", 9), ("beta", 8)]))?;
    check!(first.id != second.id);
    check!(eq; first.chunks[0].len(), second.chunks[0].len());
    let current = store
        .snapshot()
        .journal_object(PHASE, KEY)?
        .ok_or("current")?;
    check!(eq; current.object_id(), second.id.as_str());
    let restreamed = stream(&current)?;
    check!(eq; restreamed[0].payload(), &json!({"rows": 9}));
    let key = object_chunk_key(&first.id, 0)?;
    let foreign = chunk_record(&second.id, 0, &second.chunks[0]);
    write(
        &store,
        &[
            (key.clone(), serde_json::to_vec(&foreign)?),
            alias_pair(&first.id)?,
        ],
    )?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalDescriptor { .. })
    ));
    let stale = chunk_record(&first.id, 0, &second.chunks[0]);
    write(
        &store,
        &[(key, serde_json::to_vec(&stale)?), alias_pair(&first.id)?],
    )?;
    check!(matches!(
        read(&store),
        Err(StoreError::JournalChunkDigest { index: 0, declared, computed, .. })
            if declared == object_digest(&first.chunks[0])
                && computed == object_digest(&second.chunks[0])
    ));
    let untouched = captured.journal_object(PHASE, KEY)?.ok_or("captured")?;
    check!(eq; untouched.object_id(), first.id.as_str());
    let streamed = stream(&untouched)?;
    check!(eq; streamed.len(), 2);
    check!(eq; streamed[0].payload(), &json!({"rows": 1}));
    check!(eq; streamed[1].payload(), &json!({"rows": 2}));
    write(&store, &[alias_pair(&second.id)?])?;
    let restored = store
        .snapshot()
        .journal_object(PHASE, KEY)?
        .ok_or("restored")?;
    check!(eq; restored.object_id(), second.id.as_str());
    let reread = stream(&restored)?;
    check!(eq; reread[0].payload(), &json!({"rows": 9}));
    Ok(())
}

#[path = "journal_recovery_tests.rs"]
mod recovery;

#[path = "journal_object_bounded_tests.rs"]
mod bounded;

#[path = "context_object_tests.rs"]
mod context_object_tests;
