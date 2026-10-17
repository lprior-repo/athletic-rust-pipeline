use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Archive {
    schema: String,
    store: StoreIdentity,
    selected_snapshot_sequence: u64,
    source_store_lifecycle: u32,
    current_input_token: Vec<u8>,
    metadata: Vec<(Vec<u8>, Vec<u8>)>,
    records: Vec<(Vec<u8>, Vec<u8>)>,
    receipts: Vec<(Vec<u8>, Vec<u8>)>,
    physical: Vec<PhysicalRow>,
    external: Vec<ExternalPrefix>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhysicalRow {
    table: Table,
    key: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalPrefix {
    root: census_store::RetainedCaptureRootV1,
    rows: Vec<JournalRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalRow {
    key: Vec<u8>,
    value: Vec<u8>,
}

pub(super) fn omit_latest(original: &[u8]) -> TestResult<Vec<u8>> {
    check!(original.len() <= 4 * 1024 * 1024);
    let mut archive: Archive = serde_json::from_slice(original)?;
    check!(eq; serde_json::to_vec(&archive)?, original);
    check!(archive.records.len() <= 4096);
    let mut records = Vec::new();
    for (index, (key, raw)) in archive.records.iter().enumerate() {
        if !key.starts_with(b"roster_source_issuance/v1/reviews/") {
            continue;
        }
        let value: Value = serde_json::from_slice(raw)?;
        if let Some(ordinal) = value.pointer("/basis/next_ordinal").and_then(Value::as_u64) {
            records.push((ordinal, index));
        }
    }
    records.sort_unstable();
    check!(eq; records.len(), 2);
    let (ordinal, index) = records
        .last()
        .copied()
        .ok_or("controlled competing review")?;
    check!(eq; ordinal, 2);
    let anchors = json!({"head":archive.metadata,"token":archive.current_input_token,"physical":archive.physical,"external":archive.external,"receipts":archive.receipts});
    archive.records.remove(index);
    check!(eq; json!({"head":archive.metadata,"token":archive.current_input_token,"physical":archive.physical,"external":archive.external,"receipts":archive.receipts}), anchors);
    Ok(serde_json::to_vec(&archive)?)
}
