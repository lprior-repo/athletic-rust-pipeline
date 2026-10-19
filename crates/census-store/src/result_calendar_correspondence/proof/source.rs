use super::super::*;
use super::{capture, retained};
use crate::{FrozenCurrentEvidenceSubjectAttachmentReferencesV1, FrozenPublicIdentityBundleV2};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayContinuationV1,
};
use census_domain::model::StoreIdentity;

pub struct ResultCalendarSourceProofV1 {
    pub(super) bytes: Vec<u8>,
    pub(super) reference: AttachmentObjectReferenceV1,
}
impl ResultCalendarSourceProofV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn reference(&self) -> &AttachmentObjectReferenceV1 {
        &self.reference
    }
    pub fn into_parts(self) -> (AttachmentObjectReferenceV1, Vec<u8>) {
        (self.reference, self.bytes)
    }
}

pub struct ReplayedResultCalendarCorrespondencesV1<'a> {
    pub(super) ledger: super::super::issuance::ledger::Ledger,
    pub(super) source: retained::Source<'a>,
    pub(super) attachments: FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'a>,
    pub(super) attachment_json: &'a serde_json::value::RawValue,
    pub(super) attachment_reference: AttachmentObjectReferenceV1,
    pub(super) continuation: AttachmentReplayContinuationV1,
    pub(super) snapshot_sequence: u64,
}

impl ReplayedResultCalendarCorrespondencesV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.ledger.store
    }
    pub fn inventory(&self) -> &ResultCalendarInventoryV1 {
        &self.ledger.inventory
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }
    pub fn records(&self) -> &[ResultCalendarReviewedRecordV1] {
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
    ) -> StoreResult<impl Iterator<Item = &'a ResultCalendarReviewedRecordV1> + use<'a>> {
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
    pub fn retained_attachment_closure(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&serde_json::value::RawValue, &AttachmentObjectReferenceV1)> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            self.attachments.require_original_grant(grant)?;
            grant.charge_steps(1).map_err(grant_error)?;
            Ok((self.attachment_json, &self.attachment_reference))
        })
    }
    pub fn reviewed_source_roots(
        &self,
        record: &ResultCalendarDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(crate::RetainedCaptureRootV1, crate::RetainedCaptureRootV1)> {
        guarded(grant, |grant| {
            self.require_original_grant(grant)?;
            for (stored, materials) in self.ledger.records.iter().zip(&self.ledger.contexts) {
                grant.charge_steps(1).map_err(grant_error)?;
                if &stored.record_digest != record {
                    continue;
                }
                let source = &stored.command.source;
                let result = super::super::source::root(
                    materials,
                    &source.result.result_root,
                    crate::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
                    grant,
                )?;
                let documentary = super::super::source::root(
                    materials,
                    &source.calendar.documentary_root,
                    crate::RetainedCaptureRootSchemaV1::ParticipationPacketV1,
                    grant,
                )?;
                return Ok((result, documentary));
            }
            Err(Error::Corrupt("reviewed source record absent").into())
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
}

impl CurrentResultCalendarCorrespondencesV1<'_, '_> {
    pub fn freeze_closure(
        &self,
        identity: &FrozenPublicIdentityBundleV2,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ResultCalendarSourceProofV1> {
        guarded(grant, |grant| capture::freeze(self, identity, grant))
    }
}
