use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_store::FrozenEvidenceSubjectAttachmentBundleV1;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::export::qualification::participation) struct AttachmentClosureDescriptor {
    object: String,
    reference: AttachmentObjectReferenceV1,
}

pub(in crate::export::qualification::participation) fn capture_attachment(
    raw: &serde_json::value::RawValue,
    reference: &AttachmentObjectReferenceV1,
    directory: &Path,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<AttachmentClosureDescriptor> {
    grant
        .charge_steps(1)
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::from)
        .map_err(census_store::StoreError::from)?;
    Ok(AttachmentClosureDescriptor {
        object: super::objects::install(&raw, directory, 3)?,
        reference: reference.clone(),
    })
}

pub(in crate::export::qualification::participation) fn replay_attachment(
    descriptor: &AttachmentClosureDescriptor,
    directory: &Path,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<FrozenEvidenceSubjectAttachmentBundleV1> {
    grant
        .charge_steps(1)
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::from)
        .map_err(census_store::StoreError::from)?;
    let bytes = super::objects::load_bytes(directory, &descriptor.object, 3)?;
    Ok(FrozenEvidenceSubjectAttachmentBundleV1::from_owned_bytes(
        bytes,
        descriptor.reference.clone(),
        grant,
    )?)
}
