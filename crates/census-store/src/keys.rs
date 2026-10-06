use serde::Deserialize;

use super::{Store, StoreError, StoreResult, Table, MAX_ID_BYTES};

pub(super) const DERIVED_SEQUENCE: u64 = 0;

const DERIVED_NAMESPACE: &[u8] = b"derived";
const GENERATION_BYTES: usize = 8;
const SEQUENCE_BYTES: usize = 8;

#[derive(Debug, Clone, Copy)]
pub(super) struct Layout {
    pub(super) derived_generation: u64,
}

impl Layout {
    pub(super) fn prefix(self, table: Table) -> Vec<u8> {
        if table.generation_partitioned() {
            derived_generation_prefix(table, self.derived_generation)
        } else {
            observation_prefix(table)
        }
    }
}

pub(super) fn observation_prefix(table: Table) -> Vec<u8> {
    let mut out = Vec::with_capacity(table.file().len().saturating_add(1));
    out.extend_from_slice(table.file().as_bytes());
    out.push(0);
    out
}

pub(super) fn derived_table_prefix(table: Table) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        DERIVED_NAMESPACE
            .len()
            .saturating_add(table.file().len())
            .saturating_add(2),
    );
    out.extend_from_slice(DERIVED_NAMESPACE);
    out.push(0);
    out.extend_from_slice(table.file().as_bytes());
    out.push(0);
    out
}

pub(super) fn derived_generation_prefix(table: Table, generation: u64) -> Vec<u8> {
    let mut out = derived_table_prefix(table);
    out.extend_from_slice(&generation.to_be_bytes());
    out.push(0);
    out
}

pub(super) fn observation_key(table: Table, id: &str, sequence: u64) -> Vec<u8> {
    let mut out = observation_prefix(table);
    out.extend_from_slice(id.as_bytes());
    out.push(0);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

pub(super) fn derived_key(table: Table, generation: u64, id: &[u8]) -> Vec<u8> {
    let mut out = derived_generation_prefix(table, generation);
    out.extend_from_slice(id);
    out
}

pub(super) fn view_derived_key(table: Table, key: &[u8]) -> Option<(u64, &[u8])> {
    let relative = key.strip_prefix(derived_table_prefix(table).as_slice())?;
    let generation_bytes: [u8; GENERATION_BYTES] =
        relative.get(..GENERATION_BYTES)?.try_into().ok()?;
    let separator = relative.get(GENERATION_BYTES)?;
    if *separator != 0 {
        return None;
    }
    let id = relative.get(GENERATION_BYTES + 1..)?;
    if id.is_empty() {
        return None;
    }
    Some((u64::from_be_bytes(generation_bytes), id))
}

fn derived_label(rest: &[u8]) -> Option<String> {
    let table_end = rest.iter().position(|&b| b == 0)?;
    let table = rest.get(..table_end)?;
    let after = rest.get(table_end.checked_add(1)?..)?;
    let generation_bytes: [u8; GENERATION_BYTES] =
        after.get(..GENERATION_BYTES)?.try_into().ok()?;
    if after.get(GENERATION_BYTES) != Some(&0) {
        return None;
    }
    let id = after.get(GENERATION_BYTES + 1..)?;
    Some(format!(
        "derived:{}:{}#{}",
        String::from_utf8_lossy(table),
        u64::from_be_bytes(generation_bytes),
        String::from_utf8_lossy(id)
    ))
}

pub(super) fn view_observation_key(key: &[u8]) -> Option<(&[u8], &[u8], u64)> {
    let sequence_start = key.len().checked_sub(SEQUENCE_BYTES)?;
    let separator = sequence_start.checked_sub(1)?;
    if key.get(separator) != Some(&0) {
        return None;
    }
    let text_bytes = key.get(..separator)?;
    let table_end = text_bytes.iter().position(|&b| b == 0)?;
    let sequence_bytes: [u8; SEQUENCE_BYTES] = key.get(sequence_start..)?.try_into().ok()?;
    let id_start = table_end.checked_add(1)?;
    Some((
        key.get(..table_end)?,
        key.get(id_start..separator)?,
        u64::from_be_bytes(sequence_bytes),
    ))
}

pub(super) fn key_label(key: &[u8]) -> String {
    if let Some(rest) = key.strip_prefix(DERIVED_NAMESPACE) {
        if let Some(rest) = rest.strip_prefix(&[0]) {
            if let Some(label) = derived_label(rest) {
                return label;
            }
        }
    }
    match view_observation_key(key) {
        Some((table, id, sequence)) => format!(
            "{}:{}#{sequence}",
            String::from_utf8_lossy(table),
            String::from_utf8_lossy(id),
        ),
        None => String::from_utf8_lossy(key).replace('\0', ":"),
    }
}

#[derive(Deserialize)]
struct ObservationId {
    id: String,
}

pub(super) fn observation_id(bytes: &[u8]) -> StoreResult<String> {
    let parsed: ObservationId =
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
