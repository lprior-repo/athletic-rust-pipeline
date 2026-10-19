use super::{
    ledger::{declared, find, VerifiedLedger},
    material::Material,
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::{RetainedCaptureLeafV1, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayContinuationV1,
    AttachmentReplayGrantV1, RetainedDocumentSubjectReferenceV1, ReviewedReferenceRecordV1,
};

pub(crate) struct ReviewedAttachmentSourceV1<'a> {
    records: &'a [ReviewedReferenceRecordV1],
    introductions: &'a [Vec<Material>],
    continuation: &'a AttachmentReplayContinuationV1,
}

impl<'a> ReviewedAttachmentSourceV1<'a> {
    pub(super) fn new<P>(
        ledger: &'a VerifiedLedger<P>,
        continuation: &'a AttachmentReplayContinuationV1,
    ) -> Self {
        Self {
            records: &ledger.records,
            introductions: &ledger.introductions,
            continuation,
        }
    }

    pub(crate) fn records(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&'a [ReviewedReferenceRecordV1]> {
        self.continuation.require(grant).map_err(Error::Domain)?;
        Ok(self.records)
    }

    pub(crate) fn bind(
        &self,
        record: &ReviewedReferenceRecordV1,
        subject: &RetainedDocumentSubjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<ReviewedAttachmentSubjectV1<'a>>> {
        self.continuation.require(grant).map_err(Error::Domain)?;
        grant
            .charge_steps(self.records.len())
            .map_err(Error::Domain)?;
        let mut records = self.records.iter().filter(|original| {
            original.command.operation == record.command.operation
                && original.record_digest == record.record_digest
        });
        let Some(original) = records.next() else {
            return Ok(None);
        };
        if records.next().is_some() {
            return Err(Error::Corrupt("ambiguous original attachment record").into());
        }
        let subjects = original.command.subjects.as_slice();
        grant.charge_steps(subjects.len()).map_err(Error::Domain)?;
        let mut subjects = subjects.iter().filter(|candidate| *candidate == subject);
        let Some(subject) = subjects.next() else {
            return Ok(None);
        };
        if subjects.next().is_some() {
            return Err(Error::Corrupt("ambiguous original attachment subject").into());
        }
        Ok(Some(ReviewedAttachmentSubjectV1 {
            record: original,
            subject,
            introductions: self.introductions,
            continuation: self.continuation,
        }))
    }
}

pub(crate) struct ReviewedAttachmentSubjectV1<'a> {
    record: &'a ReviewedReferenceRecordV1,
    subject: &'a RetainedDocumentSubjectReferenceV1,
    introductions: &'a [Vec<Material>],
    continuation: &'a AttachmentReplayContinuationV1,
}

impl<'a> ReviewedAttachmentSubjectV1<'a> {
    pub(crate) fn capture(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&'a RetainedCaptureLeafV1> {
        match self.material(&self.subject.capture, grant)? {
            Material::Retained { leaf } => Ok(leaf),
            Material::Inline { .. } => {
                Err(Error::Corrupt("original subject capture is not retained").into())
            }
        }
    }

    pub(crate) fn body(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&'a AttachmentObjectReferenceV1> {
        Ok(self.material(&self.subject.body, grant)?.reference())
    }

    fn material(
        &self,
        digest: &AttachmentDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&'a Material> {
        self.continuation.require(grant).map_err(Error::Domain)?;
        let material = find(self.introductions, digest, grant)?
            .ok_or(Error::Corrupt("original attachment material absent"))?;
        if !declared(
            self.record.command.reviewed_context.objects.as_slice(),
            material.reference(),
            grant,
        )? {
            return Err(Error::Corrupt("original attachment material reference differs").into());
        }
        Ok(material)
    }
}
