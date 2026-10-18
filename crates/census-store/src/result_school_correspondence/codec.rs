use super::*;
use serde::{de::DeserializeOwned, Serialize};
pub(super) mod bounded;
pub(super) mod json_bytes;

pub(super) fn encode(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(grant_error)?;
    let mut measure = Measure { bytes: 0 };
    serde_json::to_writer(&mut measure, value).map_err(|_| Error::BudgetExceeded)?;
    grant.charge_bytes(measure.bytes).map_err(grant_error)?;
    grant
        .charge_steps(measure.bytes.div_ceil(512))
        .map_err(grant_error)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(measure.bytes)
        .map_err(|_| Error::BudgetExceeded)?;
    serde_json::to_writer(&mut bytes, value).map_err(|_| Error::Corrupt("record encoding"))?;
    if bytes.len() != measure.bytes {
        return Err(Error::Corrupt("encoded size changed").into());
    }
    Ok(bytes)
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<T> {
    if raw.len() > MAX_RESULT_SCHOOL_RECORD_BYTES {
        return Err(Error::BudgetExceeded.into());
    }
    grant.charge_bytes(raw.len()).map_err(grant_error)?;
    grant.charge_steps(1).map_err(grant_error)?;
    let value: T = serde_json::from_slice(raw).map_err(|_| Error::Corrupt("record decoding"))?;
    if encode(&value, grant)? != raw {
        return Err(Error::Corrupt("noncanonical record").into());
    }
    Ok(value)
}

pub(super) fn digest(
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultSchoolDigestV1> {
    grant
        .charge_steps(raw.len().div_ceil(512))
        .map_err(grant_error)?;
    use sha2::{Digest, Sha256};
    ResultSchoolDigestV1::parse(&format!("{:x}", Sha256::digest(raw))).map_err(grant_error)
}

struct Measure {
    bytes: usize,
}
impl std::io::Write for Measure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("school record size overflow"))?;
        if self.bytes > MAX_RESULT_SCHOOL_RECORD_BYTES {
            return Err(std::io::Error::other("school record size limit"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn decode_entry(
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::RecordEnvelope> {
    if raw.len() > census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes {
        return Err(Error::BudgetExceeded.into());
    }
    grant.charge_bytes(raw.len()).map_err(grant_error)?;
    grant.charge_steps(1).map_err(grant_error)?;
    let value: wire::RecordEnvelope =
        serde_json::from_slice(raw).map_err(|_| Error::Corrupt("reviewed entry decoding"))?;
    bounded::require_canonical(&value, raw, grant)?;
    source::validate_inventory(&value.record.command.source, &value.context, grant)?;
    Ok(value)
}
