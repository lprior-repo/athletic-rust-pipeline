use super::super::{refused, MAX_PACKET, PHASE};
use crate::journal::{chunk_key, chunk_value, manifest_value, split_chunks};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use serde::Serialize;
use serde_json::value::RawValue;

#[derive(Serialize)]
struct Envelope<'a> {
    key: &'a str,
    at: &'a str,
    payload: &'a RawValue,
}

pub(super) fn records(key: &str, at: &str, packet: &[u8]) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    records_for(PHASE, key, at, packet)
}

pub(in crate::acquisition) fn records_for(
    phase: &str,
    key: &str,
    at: &str,
    packet: &[u8],
) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let text =
        std::str::from_utf8(packet).map_err(|_| refused("public school intake packet encoding"))?;
    let raw = RawValue::from_string(text.to_string())
        .map_err(|_| refused("public school intake raw payload"))?;
    let value = serde_json::to_vec(&Envelope {
        key,
        at,
        payload: &raw,
    })
    .map_err(|_| refused("public school intake envelope encoding"))?;
    if value.len() > MAX_PACKET {
        return Err(refused("public school intake envelope limit"));
    }
    let root = Store::journal_key(phase, key);
    let records = if value.len() <= MAX_JOURNAL_VALUE_BYTES {
        vec![(root, value)]
    } else {
        chunked(key, root, &value)?
    };
    validate_bounds(&records)?;
    Ok(records)
}

fn chunked(key: &str, root: Vec<u8>, value: &[u8]) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let chunks = split_chunks(value);
    let total = chunks.len();
    let mut records = vec![(root.clone(), manifest_value(key, total, value.len())?)];
    for (index, chunk) in chunks.iter().enumerate() {
        records.push((
            chunk_key(&root, index)?,
            chunk_value(key, index, total, chunk)?,
        ));
    }
    Ok(records)
}

fn validate_bounds(records: &[(Vec<u8>, Vec<u8>)]) -> StoreResult<()> {
    if u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?
        > crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_PHYSICAL_ROWS
    {
        return Err(refused("public school intake physical row limit"));
    }
    let mut bytes = 0_usize;
    for (key, value) in records {
        if key.len() > crate::MAX_JOURNAL_KEY_BYTES || value.len() > MAX_JOURNAL_VALUE_BYTES {
            return Err(refused("public school intake physical record limit"));
        }
        bytes = bytes
            .checked_add(key.len())
            .and_then(|size| size.checked_add(value.len()))
            .ok_or(StoreError::CounterOverflow)?;
    }
    if bytes > MAX_PACKET {
        return Err(refused("public school intake physical bytes limit"));
    }
    Ok(())
}
