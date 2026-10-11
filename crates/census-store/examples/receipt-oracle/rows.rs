use census_store::{NativeEffectRow, Table};
use std::collections::BTreeMap;

pub fn row_index(rows: &[NativeEffectRow]) -> BTreeMap<(String, String), String> {
    rows.iter()
        .map(|row| {
            (
                (row.table.file().to_string(), hex(&row.key)),
                row.digest.clone(),
            )
        })
        .collect()
}

pub fn group_rows(rows: &[NativeEffectRow]) -> BTreeMap<(String, String, String), usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        let group = (
            row.table.file().to_string(),
            decode_row_key(&row.table, &row.key),
            row.digest.clone(),
        );
        let count = counts.entry(group).or_insert(0usize);
        *count = count.saturating_add(1);
    }
    counts
}

pub fn decode_row_key(table: &Table, key: &[u8]) -> String {
    let mut prefix = table.file().as_bytes().to_vec();
    prefix.push(0);
    let Some(relative) = key.strip_prefix(prefix.as_slice()) else {
        return hex(key);
    };
    if relative.len() < 9 {
        return hex(key);
    }
    let (identity, tail) = relative.split_at(relative.len().saturating_sub(8));
    let Some(identity) = identity.strip_suffix(&[0u8]) else {
        return hex(key);
    };
    let Ok(id) = std::str::from_utf8(identity) else {
        return hex(key);
    };
    let mut sequence = [0u8; 8];
    sequence.copy_from_slice(tail);
    format!("{id}@{}", u64::from_be_bytes(sequence))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
