use super::RegionContractRegistryError;
use crate::{StoreError, StoreResult};
use census_domain::model::ContractRefV1;
use std::fmt::Write;

pub(super) fn descriptor(reference: &ContractRefV1) -> StoreResult<String> {
    let mut key = String::new();
    key.try_reserve_exact(96)
        .map_err(|_| RegionContractRegistryError::AllocationFailed)?;
    write!(
        &mut key,
        "region-contracts:v1/{}/{}",
        reference.revision(),
        reference.digest()
    )
    .map_err(|_| StoreError::Invariant {
        detail: "region key encoding failed".to_string(),
    })?;
    Ok(key)
}

pub(super) fn chunk(key: &str, index: usize) -> StoreResult<Vec<u8>> {
    crate::journal::chunk_key(key.as_bytes(), index)
}

pub(super) fn chunk_prefix(key: &str) -> StoreResult<Vec<u8>> {
    let mut prefix = chunk(key, 0)?;
    let length = prefix
        .len()
        .checked_sub(4)
        .ok_or(StoreError::CounterOverflow)?;
    prefix.truncate(length);
    Ok(prefix)
}

pub(super) fn ordinal(key: &str, raw: &[u8]) -> Option<usize> {
    let expected = key.len().checked_add(5)?;
    if raw.len() != expected {
        return None;
    }
    let (logical, index) = crate::journal::parse_chunk_key(raw)?;
    (logical == key.as_bytes()).then_some(index)
}
