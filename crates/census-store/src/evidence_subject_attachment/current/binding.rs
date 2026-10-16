use super::*;

pub(super) fn bind<'a, 's>(
    owner: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
    expected: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<CurrentEvidenceSubjectAttachmentV1<'a, 's>>> {
    let Some(parts) = super::query::bind(&owner.ledger, owner.accepted.view(), expected, grant)?
    else {
        return Ok(None);
    };
    Ok(Some(CurrentEvidenceSubjectAttachmentV1 {
        view: owner.view,
        ledger: &owner.ledger,
        accepted: owner.accepted.view(),
        record: parts.record,
        subject: parts.subject,
        receipt: parts.receipt,
    }))
}
