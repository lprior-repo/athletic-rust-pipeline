use super::super::material::ReadAttachmentContextObjectV1;
use super::*;
use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;

impl<'v, 's> CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's> {
    pub fn read_active_context_object_v1(
        &self,
        subject: &RetainedDocumentSubjectReferenceV1,
        reference: &AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<ReadAttachmentContextObjectV1<'_>>> {
        let result = self.read_active_context_object(subject, reference, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }

    fn read_active_context_object(
        &self,
        subject: &RetainedDocumentSubjectReferenceV1,
        reference: &AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<ReadAttachmentContextObjectV1<'_>>> {
        let Some(bound) = self.bind_exact_subject(subject, grant)? else {
            return Ok(None);
        };
        bound.read_context_object_v1(reference, grant)
    }
}
