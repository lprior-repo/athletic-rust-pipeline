use super::super::material;
use super::*;
use crate::Receipt;
use census_domain::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentObjectReferenceV1, AttachmentReplayContinuationV1,
    AttachmentTargetV1, ReviewedReferenceRecordV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::RegistryHead;

pub struct CurrentEvidenceSubjectAttachmentV1<'a, 's> {
    pub(super) view: &'a StoreSnapshot<'s>,
    pub(super) ledger: &'a Ledger<'s>,
    pub(super) accepted: &'a AcceptedPublicIdentitySnapshot,
    pub(super) record: &'a ReviewedReferenceRecordV1,
    pub(super) subject: &'a RetainedDocumentSubjectReferenceV1,
    pub(super) receipt: &'a Receipt,
    pub(super) continuation: &'a AttachmentReplayContinuationV1,
}

impl<'a, 's> CurrentEvidenceSubjectAttachmentV1<'a, 's> {
    pub fn snapshot(&self) -> &StoreSnapshot<'s> {
        self.view
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
        &self.ledger.inventory
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        self.accepted.store_identity()
    }
    pub fn accepted_public_head(&self) -> &RegistryHead {
        self.accepted.head()
    }
    pub fn accepted_public_identities(&self) -> &crate::AcceptedPublicIdentitySnapshot {
        self.accepted
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.continuation
            .require(grant)
            .map_err(|source| Error::Domain(source).into())
    }
    pub fn read_context_object_v1(
        &self,
        reference: &AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<material::ReadAttachmentContextObjectV1<'a>>> {
        self.require_original_grant(grant)?;
        let result = self
            .ledger
            .read_approved_context_object(self.record, reference, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
