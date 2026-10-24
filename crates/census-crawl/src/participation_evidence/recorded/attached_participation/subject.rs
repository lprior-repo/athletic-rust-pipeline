use super::*;
use crate::participation_evidence::{
    hash, DocumentSubjectV1, IndependentParticipationProofV1, SubjectOwnershipV1,
    VerifiedIndependentParticipationWindowV1,
};
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
    let proof = checked.status_proof();
    if document != checked.subject() || proof.locator() != document.locator() {
        return Err(invalid("attached status original subject differs").into());
    }
    require_subject(item, document, proof, grant)
}

pub(super) fn require_window(
    item: &ActiveAttachmentSubjectV1<'_>,
    checked: &VerifiedIndependentParticipationWindowV1,
    grant: &mut Grant,
) -> CrawlResult<()> {
    let SubjectOwnershipV1::DocumentOnly(document) = checked.ownership() else {
        return Err(invalid("attached window requires original documentary ownership").into());
    };
    if document != checked.subject() {
        return Err(invalid("attached window original subject differs").into());
    }
    require_subject(item, document, checked.boundary_proof(), grant)
}

const DOCUMENT_SUBJECT_PREFIX_BYTES: usize = "document_subject_v1".len() + 5;

fn require_subject(
    item: &ActiveAttachmentSubjectV1<'_>,
    document: &DocumentSubjectV1,
    proof: &IndependentParticipationProofV1,
    grant: &mut Grant,
) -> CrawlResult<()> {
    let subject = item.subject();
    steps(grant, 10)?;
    let preimage = [
        document.document_namespace(),
        document.native_document_id(),
        document.document_url(),
        document.body_sha256(),
        document.locator(),
    ]
    .into_iter()
    .try_fold(DOCUMENT_SUBJECT_PREFIX_BYTES, |held, value| {
        held.checked_add(value.len())
            .ok_or(census_store::StoreError::CounterOverflow)
    })?;
    bytes(
        grant,
        preimage
            .checked_add(proof.locator().len())
            .and_then(|value| value.checked_add(128))
            .ok_or(census_store::StoreError::CounterOverflow)?,
    )?;
    if subject.role != AttachmentSubjectRoleV1::AthleteStatus
        || subject.namespace.as_str() != document.document_namespace()
        || subject.native_id.as_ref().map(|value| value.as_str())
            != Some(document.native_document_id())
        || subject.body.as_str() != document.body_sha256()
        || subject.body.as_str() != proof.capture()
        || subject.interpretation.as_str() != proof.interpretation()
        || subject.locator.as_str() != hash(proof.locator().as_bytes())
        || subject.subject.as_str() != document.reference()
    {
        return Err(
            invalid("attached status semantic subject differs from retained authority").into(),
        );
    }
    Ok(())
}
