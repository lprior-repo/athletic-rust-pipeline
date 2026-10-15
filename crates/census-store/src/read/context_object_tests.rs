use super::*;

use crate::journal::context::{CONTEXT_RECORDS, CONTEXT_RECORD_BYTES, CONTEXT_TOTAL_BYTES};

fn record_with_length(length: usize) -> TestResult<Value> {
    let empty = record("context", json!(""));
    let overhead = serde_json::to_vec(&empty)?.len() + 1;
    let text = "x".repeat(length.checked_sub(overhead).ok_or("record too short")?);
    Ok(record("context", json!(text)))
}

#[test]
fn context_reader_accepts_exact_record_byte_boundary() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for length in [CONTEXT_RECORD_BYTES - 1, CONTEXT_RECORD_BYTES] {
        let entry = record_with_length(length)?;
        let staged = stage_records(&store, std::slice::from_ref(&entry))?;
        let object =
            crate::read::read_context_object(&store.db.snapshot(), &store.journal, &staged.id)?;
        check!(eq; object.total_bytes(), length);
        check!(eq; object.record_count(), 1);
        let mut rows = object.records();
        let row = rows.next().ok_or("context row")??;
        check!(eq; row.length(), length);
        check!(eq; row.entry().key.as_str(), "context");
        check!(eq; row.payload(), entry.get("payload").ok_or("expected payload")?);
        check!(rows.next().is_none());
    }
    Ok(())
}

#[test]
fn context_reader_refuses_oversized_record_before_loading_chunks_without_changing_legacy_limit(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let length = CONTEXT_RECORD_BYTES + 1;
    let entry = record_with_length(length)?;
    let staged = stage_records(&store, std::slice::from_ref(&entry))?;
    let historical = read(&store)?.ok_or("legacy object")?;
    let row = historical.records().next().ok_or("legacy record")??;
    check!(eq; row.length(), length);
    check!(eq; row.payload(), entry.get("payload").ok_or("expected payload")?);

    write(
        &store,
        &[(object_chunk_key(&staged.id, 0)?, b"invalid chunk".to_vec())],
    )?;
    let sequence = store.snapshot().sequence();
    match crate::read::read_context_object(&store.db.snapshot(), &store.journal, &staged.id) {
        Err(StoreError::RowTooLarge { bytes, max }) => {
            check!(eq; bytes, length);
            check!(eq; max, CONTEXT_RECORD_BYTES);
        }
        other => {
            return Err(format!("expected context record admission refusal, got {other:?}").into())
        }
    }
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn context_reader_refuses_aggregate_limits_before_loading_missing_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("context", 1)]))?;
    let variants = [
        (CONTEXT_TOTAL_BYTES + 1, 1),
        (staged.payload.len(), CONTEXT_RECORDS + 1),
    ];
    for (bytes, records) in variants {
        let chunks = crate::journal::expected_chunk_count(bytes);
        let mut manifest = manifest_record(&staged.id, &staged);
        manifest["total_bytes"] = json!(bytes);
        manifest["chunks"] = json!(chunks);
        manifest["chunk_digests"] = json!(vec![object_digest(b""); chunks]);
        manifest["records"] = json!(records);
        manifest["index_pages"] = json!(records.div_ceil(OBJECT_INDEX_PAGE_ENTRIES));
        write(
            &store,
            &[(
                object_manifest_key(&staged.id)?,
                serde_json::to_vec(&manifest)?,
            )],
        )?;
        match crate::read::read_context_object(&store.db.snapshot(), &store.journal, &staged.id) {
            Err(StoreError::JournalDescriptor { phase, key, detail }) => {
                check!(eq; phase, "authority_context_v1");
                check!(eq; key, staged.id);
                check!(eq; detail, "authority context exceeds retained input limits");
            }
            other => {
                return Err(format!("expected context aggregate refusal, got {other:?}").into())
            }
        }
    }
    Ok(())
}

struct SerializationProbe(std::cell::Cell<usize>);

impl serde::Serialize for SerializationProbe {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.set(self.0.get() + 1);
        Err(serde::ser::Error::custom("reserved payload was serialized"))
    }
}

fn reserved(result: StoreResult<()>) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) => {
            check!(eq; detail, "journal object namespace requires its immutable owned writer");
        }
        other => return Err(format!("expected immutable namespace refusal, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn generic_writes_cannot_overwrite_j2_rows_through_phase_or_key_separators() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &entries(&[("original", 1)]))?;
    let sequence = store.snapshot().sequence();
    let probe = SerializationProbe(std::cell::Cell::new(0));
    let physical = [
        object_manifest_key(&staged.id)?,
        object_index_key(&staged.id, 0)?,
        object_chunk_key(&staged.id, 0)?,
    ];
    for physical in physical {
        for split in physical
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == 0).then_some(index))
        {
            let phase = std::str::from_utf8(&physical[..split])?;
            let key = std::str::from_utf8(&physical[split + 1..])?;
            check!(eq; Store::journal_key(phase, key), physical);
            reserved(store.journal_done(phase, key, &probe))?;
            let mut batch = store.write_batch();
            reserved(batch.journal_done(phase, key, &probe))?;
            check!(batch.is_empty());
        }
    }
    check!(eq; probe.0.get(), 0);
    check!(eq; store.snapshot().sequence(), sequence);
    let object = read(&store)?.ok_or("preserved object")?;
    check!(eq; object.object_id(), staged.id.as_str());
    let rows = stream(&object)?;
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].entry().key.as_str(), "original");
    check!(eq; rows[0].payload(), &json!({"rows": 1}));
    Ok(())
}

#[test]
fn derived_publication_cannot_write_the_reserved_j2_phase() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut stage = store.stage_derived()?;
    let sequence = store.snapshot().sequence();
    let probe = SerializationProbe(std::cell::Cell::new(0));
    reserved(stage.journal_done_once("j2", "forged", &probe))?;
    check!(eq; probe.0.get(), 0);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(!store.journal_contains("j2", "forged")?);
    Ok(())
}

#[test]
fn generic_writes_keep_neighboring_j2_named_phases_available() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for phase in ["j20", "j2-context", "j2:legacy"] {
        store.journal_done(phase, "direct", &json!({"kind": "direct"}))?;
        check!(eq; store.journal_payload(phase, "direct")?, Some(json!({"kind": "direct"})));
        let mut batch = store.write_batch();
        batch.journal_done(phase, "batch", &json!({"kind": "batch"}))?;
        batch.commit()?;
        check!(eq; store.journal_payload(phase, "batch")?, Some(json!({"kind": "batch"})));
    }
    Ok(())
}
