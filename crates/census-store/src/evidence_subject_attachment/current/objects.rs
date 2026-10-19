use super::super::material::{Material, ReadAttachmentContextObjectV1};
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
        bound.require_original_grant(grant)?;

        let objects = bound.record().command.reviewed_context.objects.as_slice();
        grant.charge_steps(objects.len()).map_err(Error::Domain)?;
        let Some(approved) = objects
            .iter()
            .find(|candidate| candidate.digest == reference.digest)
        else {
            return Ok(None);
        };
        if approved != reference {
            return Err(Error::Corrupt("active context object length differs").into());
        }

        let object = self
            .ledger
            .find_object(&reference.digest, grant)?
            .ok_or(Error::Corrupt("missing active context object"))?;
        if object.reference() != reference {
            return Err(Error::Corrupt("active context object reference differs").into());
        }
        Ok(Some(match object {
            Material::Inline { reference, .. } => ReadAttachmentContextObjectV1::Inline {
                reference,
                bytes: object.inline_bytes(grant)?,
            },
            Material::Retained { leaf } => ReadAttachmentContextObjectV1::Retained(leaf),
        }))
    }
}
