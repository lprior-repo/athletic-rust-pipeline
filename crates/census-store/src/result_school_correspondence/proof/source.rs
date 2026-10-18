use super::super::*;
use super::{capture, retained};
use crate::{FrozenCurrentEvidenceSubjectAttachmentReferencesV1, FrozenPublicIdentityBundleV2};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayContinuationV1,
};
use census_domain::model::{CanonicalSchool, StoreIdentity};

pub struct ResultSchoolSourceProofV1 {
    pub(super) bytes: Vec<u8>,
    pub(super) reference: AttachmentObjectReferenceV1,
}
impl ResultSchoolSourceProofV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.reference
    }
}

pub struct ReplayedResultSchoolCorrespondencesV1<'a> {
    pub(super) ledger: super::super::issuance::ledger::Ledger,
    pub(super) source: retained::Source<'a>,
    pub(super) attachments: FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'a>,
    pub(super) continuation: AttachmentReplayContinuationV1,
    pub(super) snapshot_sequence: u64,
}

impl ReplayedResultSchoolCorrespondencesV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.ledger.store
    }
    pub fn inventory(&self) -> &ResultSchoolInventoryV1 {
        &self.ledger.inventory
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }
    pub fn records(&self) -> &[ResultSchoolReviewedRecordV1] {
        &self.ledger.records
    }
    pub fn receipts(&self) -> &[crate::Receipt] {
        &self.ledger.receipts
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.continuation.require(grant).map_err(grant_error)
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
    pub fn attachments(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'_>> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            self.attachments.require_original_grant(grant)?;
            Ok(&self.attachments)
        })
    }
    pub fn read_retained_capture_root_v1(
        &self,
        schema: crate::RetainedCaptureRootSchemaV1,
        key: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<crate::RetainedCaptureRootReadV1>> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            crate::evidence_subject_attachment::external::read_owned(
                &retained::Combined::new(&self.source, self.attachments.source()),
                schema,
                key,
                grant,
            )
        })
    }
    pub fn physical_school(
        &self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CanonicalSchool> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            super::super::source::physical_school(&self.ledger, digest, grant)
        })
    }
    pub fn provider_school_root(
        &self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<crate::RetainedCaptureRootV1> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            super::super::source::provider_school_root(&self.ledger, digest, grant)
        })
    }
}

impl CurrentResultSchoolCorrespondencesV1<'_, '_> {
    pub fn freeze_closure(
        &self,
        identity: &FrozenPublicIdentityBundleV2,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ResultSchoolSourceProofV1> {
        guarded(grant, |grant| capture::freeze(self, identity, grant))
    }
    pub fn physical_school(
        &self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CanonicalSchool> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            super::super::source::physical_school(&self.ledger, digest, grant)
        })
    }
    pub fn provider_school_root(
        &self,
        digest: &ResultSchoolDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<crate::RetainedCaptureRootV1> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            super::super::source::provider_school_root(&self.ledger, digest, grant)
        })
    }
}
