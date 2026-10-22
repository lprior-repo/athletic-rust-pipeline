use super::RosterSourceIssuanceStoreError as Error;
use crate::StoreResult;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

pub(super) const MAX_BYTES: usize =
    census_domain::roster_affiliation_authority::MAX_ROSTER_STORE_RECORD_BYTES;

pub(super) const MAX_REVIEW_COMPOSITE_BYTES: usize =
    census_domain::roster_affiliation_authority::MAX_ROSTER_REVIEW_COMPOSITE_BYTES;

pub(super) fn encode<T: Serialize>(value: &T) -> StoreResult<Vec<u8>> {
    encode_bounded(value, MAX_BYTES)
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> StoreResult<T> {
    decode_bounded(bytes, MAX_BYTES)
}

pub(super) fn decode_physical_manifest<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> StoreResult<T> {
    decode_bounded(
        bytes,
        census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES,
    )
}

pub(super) fn encode_review_composite<T: Serialize>(value: &T) -> StoreResult<Vec<u8>> {
    encode_bounded(value, MAX_REVIEW_COMPOSITE_BYTES)
}

pub(super) fn decode_review_composite<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> StoreResult<T> {
    decode_bounded(bytes, MAX_REVIEW_COMPOSITE_BYTES)
}

fn encode_bounded<T: Serialize>(value: &T, maximum: usize) -> StoreResult<Vec<u8>> {
    encode_sized(value, measured_size(value, maximum)?)
}

fn decode_bounded<T: DeserializeOwned + Serialize>(bytes: &[u8], maximum: usize) -> StoreResult<T> {
    if bytes.len() > maximum {
        return Err(Error::BudgetExceeded.into());
    }
    let value: T = serde_json::from_slice(bytes).map_err(|_| Error::Corrupt("decoding"))?;
    if encode_bounded(&value, maximum)? != bytes {
        return Err(Error::Corrupt("noncanonical bytes").into());
    }
    Ok(value)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn encode_with_source_grant<T: Serialize>(
    value: &T,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_with_source_grant_bounded(value, grant, MAX_BYTES)
}

pub(super) fn encode_physical_manifest_with_source_grant<T: Serialize>(
    value: &T,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_with_source_grant_bounded(
        value,
        grant,
        census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES,
    )
}

pub(super) fn encode_review_composite_with_source_grant<T: Serialize>(
    value: &T,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_with_source_grant_bounded(value, grant, MAX_REVIEW_COMPOSITE_BYTES)
}

fn encode_with_source_grant_bounded<T: Serialize>(
    value: &T,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
    maximum: usize,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(super::source_error)?;
    let size = measured_size(value, maximum)?;
    grant.charge_bytes(size).map_err(super::source_error)?;
    grant.charge_steps(1).map_err(super::source_error)?;
    encode_sized(value, size)
}

fn encode_sized<T: Serialize>(value: &T, size: usize) -> StoreResult<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|error| crate::StoreError::Refused {
            detail: format!("roster review allocation: {error}"),
        })?;
    let mut encoded = Encoded {
        bytes,
        limit: size,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut encoded, value);
    if encoded.exceeded {
        return Err(Error::Corrupt("encoded size changed").into());
    }
    result.map_err(|source| crate::StoreError::Json {
        detail: "encoding roster review".into(),
        source,
    })?;
    if encoded.bytes.len() != size {
        return Err(Error::Corrupt("encoded size changed").into());
    }
    Ok(encoded.bytes)
}

fn measured_size<T: Serialize>(value: &T, maximum: usize) -> StoreResult<usize> {
    let mut measure = Measure { bytes: 0, maximum };
    let result = serde_json::to_writer(&mut measure, value);
    if measure.bytes > maximum {
        return Err(Error::BudgetExceeded.into());
    }
    result.map_err(|source| crate::StoreError::Json {
        detail: "measuring roster review".into(),
        source,
    })?;
    Ok(measure.bytes)
}

struct Measure {
    bytes: usize,
    maximum: usize,
}

impl std::io::Write for Measure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = match self.bytes.checked_add(bytes.len()) {
            Some(length) => length,
            None => usize::MAX,
        };
        if self.bytes > self.maximum {
            return Err(std::io::Error::other("roster object size cap"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Encoded {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl std::io::Write for Encoded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let length = self.bytes.len().checked_add(bytes.len());
        if !matches!(length, Some(length) if length <= self.limit) {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "roster encoding exceeds measured reservation",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bounded_capacity_tests;
