use super::*;
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::AttachmentReplayContinuationV1;
use census_domain::model::StoreIdentity;

pub struct CurrentResultSchoolCorrespondencesV1<'v, 's> {
    view: &'v StoreSnapshot<'s>,
    pub(super) attachments: &'v crate::CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
    pub(super) ledger: super::issuance::ledger::Ledger,
    continuation: AttachmentReplayContinuationV1,
}

impl<'v, 's> CurrentResultSchoolCorrespondencesV1<'v, 's> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.ledger.store
    }
    pub fn inventory(&self) -> &ResultSchoolInventoryV1 {
        &self.ledger.inventory
    }
    pub fn head(&self) -> &ResultSchoolDigestV1 {
        &self.ledger.inventory.digest
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.view.sequence()
    }
    pub fn snapshot(&self) -> &StoreSnapshot<'s> {
        self.view
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.continuation.require(grant).map_err(grant_error)
    }
    pub fn records(&self) -> &[ResultSchoolReviewedRecordV1] {
        &self.ledger.records
    }
    pub fn receipts(&self) -> &[crate::Receipt] {
        &self.ledger.receipts
    }
    pub fn active_records<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<impl Iterator<Item = &'a ResultSchoolReviewedRecordV1> + use<'a>> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            Ok(self.ledger.active_records())
        })
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn read_current_result_school_correspondences_v1<'v>(
        &'v self,
        attachments: &'v crate::CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CurrentResultSchoolCorrespondencesV1<'v, 's>> {
        guarded(grant, |grant| {
            let ledger = super::issuance::ledger::from_view(self, attachments, grant)?;
            let continuation = grant.checkpoint().map_err(grant_error)?;
            Ok(CurrentResultSchoolCorrespondencesV1 {
                view: self,
                attachments,
                ledger,
                continuation,
            })
        })
    }
}
