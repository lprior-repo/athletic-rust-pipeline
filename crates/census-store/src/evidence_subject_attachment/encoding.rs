use super::EvidenceSubjectAttachmentStoreError as Error;
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use census_domain::model::write_serialized;
use census_domain::public_identity_history::ReplayLimits;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

mod canonical;
mod sink;
pub(super) use canonical::require_canonical;

pub(super) struct EncodedObject {
    reference: AttachmentObjectReferenceV1,
    bytes: Vec<u8>,
}

impl EncodedObject {
    pub(super) fn reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.reference
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(super) fn into_parts(self) -> (AttachmentObjectReferenceV1, Vec<u8>) {
        (self.reference, self.bytes)
    }
}

pub(super) fn object(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = object_checked(value, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn object_checked(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    let bytes = measure(value, grant)?;
    object_measured(value, bytes, grant)
}

pub(super) fn measure(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    grant.ensure_available().map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let result = sink::measure(value, ReplayLimits::SMALL.total_text_bytes);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn object_measured(
    value: &impl Serialize,
    bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = encode_measured(value, bytes, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn encode_measured(
    value: &impl Serialize,
    bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.charge_bytes(bytes).map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let encoded = sink::encode(value, bytes)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let digest = AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(&encoded)))
        .map_err(Error::Domain)?;
    let reference = AttachmentObjectReferenceV1 {
        digest,
        bytes: u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?,
    };
    Ok(EncodedObject {
        reference,
        bytes: encoded,
    })
}

pub(super) fn raw_object(
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = raw_object_checked(raw, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn raw_object_checked(
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<EncodedObject> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.is_empty() || raw.len() > ReplayLimits::SMALL.total_text_bytes {
        return Err(Error::Corrupt("raw context object byte bound").into());
    }
    grant.charge_bytes(raw.len()).map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(raw.len())
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    bytes.extend_from_slice(raw);
    grant.charge_steps(1).map_err(Error::Domain)?;
    let digest = AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(&bytes)))
        .map_err(Error::Domain)?;
    Ok(EncodedObject {
        reference: AttachmentObjectReferenceV1 {
            digest,
            bytes: u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?,
        },
        bytes,
    })
}

#[cfg(test)]
mod tests;
