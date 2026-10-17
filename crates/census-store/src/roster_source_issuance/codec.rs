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

pub(super) fn encode_with_source_grant<T: Serialize>(
    value: &T,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(super::source_error)?;
    let mut measure = Measure { bytes: 0 };
    serde_json::to_writer(&mut measure, value).map_err(|_| Error::BudgetExceeded)?;
    grant.charge_bytes(measure.bytes).map_err(super::source_error)?;
    grant.charge_steps(1).map_err(super::source_error)?;
    let bytes = encode(value)?;
    if bytes.len() != measure.bytes { return Err(Error::Corrupt("encoded size changed").into()); }
    Ok(bytes)
}

struct Measure { bytes: usize }

impl std::io::Write for Measure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self.bytes.checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("roster object size overflow"))?;
        if self.bytes > MAX_BYTES { return Err(std::io::Error::other("roster object size cap")); }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
