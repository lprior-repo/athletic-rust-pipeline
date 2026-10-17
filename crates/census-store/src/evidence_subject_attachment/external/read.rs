use super::super::EvidenceSubjectAttachmentStoreError as Error;
use super::{leaf, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1};
use crate::read::{JournalRawEnvelope, RetainedJournalReadSource};
use crate::{StoreError, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

mod physical;

pub struct RetainedCaptureRootReadV1 {
    root: RetainedCaptureRootV1,
    envelope: JournalRawEnvelope,
}

impl RetainedCaptureRootReadV1 {
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
    pub fn payload(&self) -> &RawValue {
        self.envelope.payload()
    }
}

impl StoreSnapshot<'_> {
    pub fn read_retained_capture_root_v1(
        &self,
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = owned(self, schema, key, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

pub(in super::super) fn owned(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
    let completed = complete(view, schema, key, grant)?;
    if let Some(root) = &completed {
        grant
            .charge_bytes(root.payload().get().len())
            .map_err(Error::Domain)?;
    }
    Ok(completed)
}

pub(super) fn read_root(
    view: &impl RetainedJournalReadSource,
    expected: &RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<RetainedCaptureRootReadV1> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = pinned(view, expected, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn pinned(
    view: &impl RetainedJournalReadSource,
    expected: &RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<RetainedCaptureRootReadV1> {
    let actual = complete(view, expected.schema, &expected.key, grant)?
        .ok_or(Error::Corrupt("external root is absent or ambiguous"))?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if actual.root() != expected {
        return Err(Error::Corrupt("external root payload reference differs").into());
    }
    Ok(actual)
}

fn complete(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<RetainedCaptureRootReadV1>> {
    let Some(envelope) = physical::read(view, schema, key, grant)? else {
        return Ok(None);
    };
    grant.charge_steps(1).map_err(Error::Domain)?;
    let raw = envelope.payload().get().as_bytes();
    let payload = AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(raw)))
            .map_err(Error::Domain)?,
        bytes: u64::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)?,
    };
    leaf::validate_root(envelope.payload(), schema, grant)?;
    let mut owned_key = String::new();
    owned_key
        .try_reserve_exact(key.len())
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    owned_key.push_str(key);
    let root = RetainedCaptureRootV1 {
        schema,
        key: owned_key,
        payload,
    };
    Ok(Some(RetainedCaptureRootReadV1 { root, envelope }))
}

#[cfg(test)]
mod tests;
