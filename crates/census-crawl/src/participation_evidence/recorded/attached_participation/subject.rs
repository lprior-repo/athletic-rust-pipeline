use super::*;
use crate::participation_evidence::{hash, SubjectOwnershipV1};
use census_domain::evidence_subject_attachment::AttachmentSubjectRoleV1;
use census_store::ActiveAttachmentSubjectV1;

pub(super) fn require(
    item: &ActiveAttachmentSubjectV1<'_>,
    checked: &VerifiedIndependentParticipationV1,
    grant: &mut Grant,
) -> CrawlResult<()> {
    let SubjectOwnershipV1::DocumentOnly(document) = checked.ownership() else {
        return Err(invalid("attached status requires original documentary ownership").into());
    };
    let subject = item.subject();
    let proof = checked.status_proof();
    steps(grant, 10)?;
    let preimage = [document.document_namespace(), document.native_document_id(),
        document.document_url(), document.body_sha256(), document.locator()]
        .into_iter().try_fold("document_subject_v1".len() + 5, |held, value| {
            held.checked_add(value.len()).ok_or(census_store::StoreError::CounterOverflow)
        })?;
    bytes(grant, preimage.checked_add(document.locator().len())
        .and_then(|value| value.checked_add(128)).ok_or(census_store::StoreError::CounterOverflow)?)?;
    if document != checked.subject()
        || subject.role != AttachmentSubjectRoleV1::AthleteStatus
        || subject.namespace.as_str() != document.document_namespace()
        || subject.native_id.as_ref().map(|value| value.as_str()) != Some(document.native_document_id())
        || subject.body.as_str() != document.body_sha256()
        || subject.body.as_str() != proof.capture()
        || subject.interpretation.as_str() != proof.interpretation()
        || subject.locator.as_str() != hash(document.locator().as_bytes())
        || subject.subject.as_str() != document.reference()
        || proof.locator() != document.locator()
    {
        return Err(invalid("attached status semantic subject differs from retained authority").into());
    }
    Ok(())
}
