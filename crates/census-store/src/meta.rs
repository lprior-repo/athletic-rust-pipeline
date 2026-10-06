use fjall::{Keyspace, OwnedWriteBatch};

use super::{StoreError, StoreResult};

pub(super) fn get_text(meta: &Keyspace, key: &str) -> StoreResult<Option<String>> {
    let Some(value) = meta
        .get(key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    std::str::from_utf8(&value)
        .map(str::to_string)
        .map(Some)
        .map_err(|_| StoreError::SchemaUnknown {
            detail: format!("{key} is not utf-8"),
        })
}

pub(super) fn put_text(batch: &mut OwnedWriteBatch, meta: &Keyspace, key: &str, value: &str) {
    batch.insert(meta, key, value.as_bytes());
}

pub(super) fn parse_u64(key: &str, text: &str) -> StoreResult<u64> {
    text.trim()
        .parse::<u64>()
        .map_err(|_| StoreError::SchemaUnknown {
            detail: format!("{key} is not a counter"),
        })
}

pub(super) fn parse_u32(key: &str, text: &str) -> StoreResult<u32> {
    text.trim()
        .parse::<u32>()
        .map_err(|_| StoreError::SchemaUnknown {
            detail: format!("{key} is not a version"),
        })
}

pub(super) fn get_u64(meta: &Keyspace, key: &str) -> StoreResult<Option<u64>> {
    get_text(meta, key)?
        .map(|text| parse_u64(key, &text))
        .transpose()
}

pub(super) fn get_u32(meta: &Keyspace, key: &str) -> StoreResult<Option<u32>> {
    get_text(meta, key)?
        .map(|text| parse_u32(key, &text))
        .transpose()
}
