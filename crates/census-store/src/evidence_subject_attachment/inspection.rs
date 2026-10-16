use super::{
    encoding::EncodedObject, guard::AttachmentCommitGuard, inputs::Inputs, ledger::Ledger, target,
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentDigestV1, AttachmentInventoryV1, AttachmentObjectReferenceV1,
    AttachmentPredecessorV1, AttachmentPublicHeadV1, AttachmentRecordReferenceV1,
    AttachmentReplayGrantV1, AttachmentTargetV1, ReviewedReferenceRecordV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::PublicAthleteId;

mod issue;

pub enum AttachmentInspectionRequestV1 {
    Attach {
        target: PublicAthleteId,
        prior: AttachmentPredecessorV1,
    },
    Revoke {
        prior: AttachmentRecordReferenceV1,
    },
}

pub struct EvidenceSubjectAttachmentInspectionV1<'s> {
    pub(super) state: Box<issue::Issued<'s>>,
    pub(super) grant: Option<AttachmentReplayGrantV1>,
}

impl EvidenceSubjectAttachmentInspectionV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.state.store
    }
    pub fn public_head(&self) -> &AttachmentPublicHeadV1 {
        &self.state.head
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.state.ledger.inventory
    }
    pub fn action(&self) -> &AttachmentActionV1 {
        &self.state.action
    }
    pub fn target(&self) -> &AttachmentTargetV1 {
        &self.state.target
    }
    pub fn accepted_target_witness(&self) -> &AttachmentDigestV1 {
        &self.state.witness
    }
    pub fn evidence_generation(&self) -> u64 {
        self.state.revision.evidence_generation()
    }
    pub fn derived_generation(&self) -> u64 {
        self.state.revision.derived_generation()
    }
    pub fn prior_record(&self) -> Option<&ReviewedReferenceRecordV1> {
        self.state
            .prior_index
            .and_then(|index| self.state.ledger.records.get(index))
    }
    pub fn issued_objects(&self) -> impl Iterator<Item = &AttachmentObjectReferenceV1> {
        self.state
            .objects
            .iter()
            .map(EncodedObject::reference)
            .chain([
                self.state.inputs.capture.reference(),
                self.state.inputs.token.reference(),
            ])
    }
}

impl Store {
    pub fn inspect_evidence_subject_attachment_v1(
        &self,
        request: AttachmentInspectionRequestV1,
    ) -> StoreResult<EvidenceSubjectAttachmentInspectionV1<'_>> {
        let mut grant = AttachmentReplayGrantV1::new();
        let result = issue::inspect(self, request, &mut grant);
        match result {
            Ok(issued) => Ok(EvidenceSubjectAttachmentInspectionV1 {
                state: Box::new(issued),
                grant: Some(grant),
            }),
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests;
