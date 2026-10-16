use super::super::EvidenceSubjectAttachmentStoreError as Error;
use super::{read, RetainedCaptureRootV1};
use crate::{StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use serde::{Deserialize, Serialize};

mod capture;
mod schema;

pub(super) fn validate_root(
    raw: &serde_json::value::RawValue,
    root: super::RetainedCaptureRootSchemaV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let slot = match root {
        super::RetainedCaptureRootSchemaV1::ParticipationPacketV1 => {
            RetainedCaptureSlotV1::ParticipationStatus
        }
        super::RetainedCaptureRootSchemaV1::ProviderTargetPacketV1 => {
            RetainedCaptureSlotV1::ProviderProfile
        }
        super::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1 => {
            RetainedCaptureSlotV1::SelfCapture
        }
        super::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1 => {
            RetainedCaptureSlotV1::ProviderSchoolDirectory
        }
    };
    schema::select(raw, root, slot, grant).map(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainedCaptureSlotV1 {
    ParticipationStatus,
    ParticipationSchedule,
    ProviderProfile,
    ProviderLinkedResults,
    SelfCapture,
    ProviderSchoolDirectory,
    ProviderSchoolRoster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainedCaptureLeafKindV1 {
    CaptureJson,
    OriginalBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedCaptureLeafV1 {
    pub version: u16,
    pub root: RetainedCaptureRootV1,
    pub slot: RetainedCaptureSlotV1,
    pub kind: RetainedCaptureLeafKindV1,
    pub object: AttachmentObjectReferenceV1,
}

pub(super) fn verify(
    view: &StoreSnapshot<'_>,
    leaf: &RetainedCaptureLeafV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = verify_checked(view, leaf, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn verify_checked(
    view: &StoreSnapshot<'_>,
    leaf: &RetainedCaptureLeafV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if leaf.version != 1 {
        return Err(Error::Corrupt("external leaf version").into());
    }
    let root = read::read_root(view, &leaf.root, grant)?;
    let selected = schema::select(root.payload(), leaf.root.schema, leaf.slot, grant)?;
    match leaf.kind {
        RetainedCaptureLeafKindV1::CaptureJson => {
            capture::require_reference(selected.get().as_bytes(), &leaf.object, grant)
        }
        RetainedCaptureLeafKindV1::OriginalBody => {
            capture::require_body(selected, &leaf.object, grant)
        }
    }
}

impl StoreSnapshot<'_> {
    pub fn verify_retained_capture_leaf_v1(
        &self,
        leaf: &RetainedCaptureLeafV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        verify(self, leaf, grant)
    }
}

#[cfg(test)]
mod tests;
