use super::super::EvidenceSubjectAttachmentStoreError as Error;
use super::{read, RetainedCaptureRootV1};
use crate::read::RetainedJournalReadSource;
use crate::{StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use serde::{Deserialize, Serialize};

pub(super) mod capture;
pub(super) mod schema;

pub(super) fn validate_root(
    raw: &serde_json::value::RawValue,
    root: super::RetainedCaptureRootSchemaV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let slot = match root {
        super::RetainedCaptureRootSchemaV1::ParticipationPacketV1 => {
            RetainedCaptureSlotV1::ParticipationStatus
        }
        super::RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1 => {
            RetainedCaptureSlotV1::ParticipationWindow
        }
        super::RetainedCaptureRootSchemaV1::ProviderTargetPacketV1 => {
            RetainedCaptureSlotV1::ProviderProfile
        }
        super::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1
        | super::RetainedCaptureRootSchemaV1::MhsaaCoopPdfPacketV1 => {
            RetainedCaptureSlotV1::SelfCapture
        }
        super::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
        | super::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1 => {
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
    ParticipationWindow,
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
    RootJson,
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

pub(crate) fn verify(
    view: &impl RetainedJournalReadSource,
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
    view: &impl RetainedJournalReadSource,
    leaf: &RetainedCaptureLeafV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if leaf.version != 1 {
        return Err(Error::Corrupt("external leaf version").into());
    }
    read::with_checked(view, &leaf.root, grant, |raw, root, grant| {
        let capture = root.select(leaf.slot)?;
        match leaf.kind {
            RetainedCaptureLeafKindV1::CaptureJson => {
                capture::require_reference(capture.raw().get().as_bytes(), &leaf.object, grant)
            }
            RetainedCaptureLeafKindV1::OriginalBody => {
                capture.body_bytes(&leaf.object, grant).map(|_| ())
            }
            RetainedCaptureLeafKindV1::RootJson => {
                if !matches!(
                    (leaf.root.schema, leaf.slot),
                    (
                        super::RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1,
                        RetainedCaptureSlotV1::ParticipationWindow
                    ) | (
                        super::RetainedCaptureRootSchemaV1::ParticipationPacketV1,
                        RetainedCaptureSlotV1::ParticipationStatus
                    ) | (
                        super::RetainedCaptureRootSchemaV1::MhsaaCoopPdfPacketV1,
                        RetainedCaptureSlotV1::SelfCapture
                    )
                ) {
                    return Err(Error::Corrupt("external root JSON kind differs").into());
                }
                capture::require_reference(raw.get().as_bytes(), &leaf.object, grant)
            }
        }
    })
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
