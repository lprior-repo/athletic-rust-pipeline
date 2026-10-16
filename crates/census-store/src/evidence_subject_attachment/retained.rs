use super::{ledger::Ledger, EvidenceSubjectAttachmentStoreError as Error};
use crate::{Receipt, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentReplayGrantV1, ReviewedReferenceRecordV1,
};

pub struct RetainedEvidenceSubjectAttachmentReferencesV1<'s> {
    ledger: Ledger<'s>,
}

impl RetainedEvidenceSubjectAttachmentReferencesV1<'_> {
    pub fn records(&self) -> &[ReviewedReferenceRecordV1] {
        &self.ledger.records
    }
    pub fn receipts(&self) -> &[Receipt] {
        &self.ledger.receipts
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.ledger.inventory
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn read_evidence_subject_attachment_references_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<RetainedEvidenceSubjectAttachmentReferencesV1<'s>> {
        grant.ensure_available().map_err(Error::Domain)?;
        grant.charge_steps(1).map_err(Error::Domain)?;
        let result = self
            .store_identity()
            .and_then(|identity| Ledger::load(self, &identity, grant));
        match result {
            Ok(ledger) => Ok(RetainedEvidenceSubjectAttachmentReferencesV1 { ledger }),
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }
}
