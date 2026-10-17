use super::*;
use crate::Receipt;
use census_domain::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentTargetV1, RetainedDocumentSubjectReferenceV1,
    ReviewedReferenceRecordV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::RegistryHead;

pub struct FrozenCurrentEvidenceSubjectAttachmentV1<'a, 'b> {
    pub(super) owner: &'a FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b>,
    pub(super) record: &'a ReviewedReferenceRecordV1,
    pub(super) subject: &'a RetainedDocumentSubjectReferenceV1,
    pub(super) receipt: &'a Receipt,
}
impl<'a, 'b> FrozenCurrentEvidenceSubjectAttachmentV1<'a, 'b> {
    pub fn source(&self) -> &FrozenRetainedCaptureSourceV1<'b> {
        self.owner.source()
    }
    pub fn subject(&self) -> &RetainedDocumentSubjectReferenceV1 {
        self.subject
    }
    pub fn target(&self) -> &AttachmentTargetV1 {
        &self.record.command.target
    }
    pub fn record(&self) -> &ReviewedReferenceRecordV1 {
        self.record
    }
    pub fn receipt(&self) -> &Receipt {
        self.receipt
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        self.owner.inventory()
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        self.owner.store_identity()
    }
    pub fn accepted_public_head(&self) -> &RegistryHead {
        self.owner.accepted_public_head()
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.owner.snapshot_sequence()
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.owner
            .continuation
            .require(grant)
            .map_err(|source| Error::Domain(source).into())
    }
}
