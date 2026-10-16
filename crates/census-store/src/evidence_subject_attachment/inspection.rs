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
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) head: AttachmentPublicHeadV1,
    pub(super) action: AttachmentActionV1,
    pub(super) target: AttachmentTargetV1,
    pub(super) witness: AttachmentDigestV1,
    pub(super) inputs: Inputs,
    pub(super) issued: Vec<EncodedObject>,
    pub(super) prior_index: Option<usize>,
    pub(super) ledger: Ledger<'s>,
    pub(super) grant: Option<AttachmentReplayGrantV1>,
}

impl EvidenceSubjectAttachmentInspectionV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn public_head(&self) -> &AttachmentPublicHeadV1 {
        &self.head
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.ledger.inventory
    }
    pub fn action(&self) -> &AttachmentActionV1 {
        &self.action
    }
    pub fn target(&self) -> &AttachmentTargetV1 {
        &self.target
    }
    pub fn accepted_target_witness(&self) -> &AttachmentDigestV1 {
        &self.witness
    }
    pub fn evidence_generation(&self) -> u64 {
        self.revision.evidence_generation()
    }
    pub fn derived_generation(&self) -> u64 {
        self.revision.derived_generation()
    }
    pub fn prior_record(&self) -> Option<&ReviewedReferenceRecordV1> {
        self.prior_index
            .and_then(|index| self.ledger.records.get(index))
    }
    pub fn issued_objects(&self) -> impl Iterator<Item = &AttachmentObjectReferenceV1> {
        self.issued.iter().map(EncodedObject::reference).chain([
            self.inputs.capture.reference(),
            self.inputs.token.reference(),
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
                revision: issued.revision,
                store: issued.store,
                head: issued.head,
                action: issued.action,
                target: issued.target,
                witness: issued.witness,
                inputs: issued.inputs,
                issued: issued.objects,
                prior_index: issued.prior_index,
                ledger: issued.ledger,
                grant: Some(grant),
            }),
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }
}
