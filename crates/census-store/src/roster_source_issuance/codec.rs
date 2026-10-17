use super::RosterSourceIssuanceStoreError as Error;
use crate::StoreResult;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

pub(super) const MAX_BYTES: usize = 64 * 1024;

pub(super) fn encode<T: Serialize>(value: &T) -> StoreResult<Vec<u8>> {
    let bytes = crate::records::codec::encode(value)?;
    if bytes.len() > MAX_BYTES { return Err(Error::BudgetExceeded.into()); }
    Ok(bytes)
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> StoreResult<T> {
    if bytes.len() > MAX_BYTES { return Err(Error::BudgetExceeded.into()); }
    let value: T = serde_json::from_slice(bytes).map_err(|_| Error::Corrupt("decoding"))?;
    if encode(&value)? != bytes { return Err(Error::Corrupt("noncanonical bytes").into()); }
    Ok(value)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
