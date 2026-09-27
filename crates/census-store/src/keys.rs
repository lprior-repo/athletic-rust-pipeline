
use serde::Deserialize;

use super::{Store, StoreError, StoreResult, Table, MAX_ID_BYTES};

#[derive(Deserialize)]
struct ObservationId<'a> {
    #[serde(borrow)]
    id: &'a str,
}

pub(super) fn table_prefix(table: Table) -> Vec<u8> {
    let mut out = Vec::with_capacity(table.file().len().saturating_add(1));
    out.extend_from_slice(table.file().as_bytes());
    out.push(0);
    out
}

pub(super) const DERIVED_SEQUENCE: u64 = 0;

pub(super) fn id_prefix(table: Table, id: &str) -> Vec<u8> {
    let mut out = table_prefix(table);
    out.extend_from_slice(id.as_bytes());
    out.push(0);
    out
}

pub(super) fn observation_key(table: Table, id: &str, sequence: u64) -> Vec<u8> {
    let mut out = id_prefix(table, id);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

pub(super) fn split_observation_key(key: &[u8]) -> Option<(&[u8], &[u8], u64)> {
    let sequence_start = key.len().checked_sub(8)?;
    let separator = sequence_start.checked_sub(1)?;
    if key.get(separator) != Some(&0) {
        return None;
    }
    let text_bytes = key.get(..separator)?;
    let table_end = text_bytes.iter().position(|&b| b == 0)?;
    let sequence_bytes: [u8; 8] = key.get(sequence_start..)?.try_into().ok()?;
    let id_start = table_end.checked_add(1)?;
    Some((
        key.get(..table_end)?,
        key.get(id_start..separator)?,
        u64::from_be_bytes(sequence_bytes),
    ))
}

pub(super) fn key_label(key: &[u8]) -> String {
    match split_observation_key(key) {
        Some((table, id, sequence)) => format!(
            "{}:{}#{sequence}",
            String::from_utf8_lossy(table),
            String::from_utf8_lossy(id),
        ),
        None => String::from_utf8_lossy(key).replace('\0', ":"),
    }
}

pub(super) fn observation_id(bytes: &[u8]) -> StoreResult<&str> {
    let parsed: ObservationId<'_> =
        serde_json::from_slice(bytes).map_err(|source| StoreError::Json {
            detail: "observation has no string id field".to_string(),
            source,
        })?;
    if parsed.id.is_empty() {
        return Err(StoreError::Invariant {
            detail: "observation id must not be empty".to_string(),
        });
    }
    if parsed.id.len() > MAX_ID_BYTES {
        return Err(StoreError::Invariant {
            detail: format!(
                "observation id of {} bytes exceeds the {MAX_ID_BYTES}-byte ceiling",
                parsed.id.len()
            ),
        });
    }
    Ok(parsed.id)
}

impl Store {
    pub(super) fn journal_key(phase: &str, key: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(phase.len().saturating_add(key.len()).saturating_add(2));
        out.extend_from_slice(phase.as_bytes());
        out.push(0);
        out.extend_from_slice(key.as_bytes());
        out
    }
}
