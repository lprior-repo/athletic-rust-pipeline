use super::{domain, invalid, ownership};
use crate::participation_evidence::recorded::DiscoveredResultAttachmentV1;
use crate::participation_evidence::result_ownership::CheckedRetainedResultRowV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, AttachmentSubjectRoleV1,
};
use census_store::{CurrentEvidenceSubjectAttachmentReferencesV1, RetainedCaptureRootV1};

pub(super) fn current(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    attached.require_original_grant(grant)?;
    if current.store_identity() != attached.store_identity()
        || current.accepted_public_head() != attached.accepted_public_head()
        || current.snapshot_sequence() != attached.snapshot_sequence()
        || current.inventory() != attached.inventory()
    {
        return Err(invalid("school source attachment owner moved"));
    }
    let actual = current
        .bind_exact_subject(attached.subject(), grant)?
        .ok_or_else(|| invalid("school source person no longer attached"))?;
    actual.require_original_grant(grant)?;
    if actual.record() != attached.record()
        || actual.target() != attached.target()
        || actual.receipt() != attached.receipt()
    {
        return Err(invalid("school source active attachment differs"));
    }
    Ok(())
}

pub(super) fn subject(
    root: &RetainedCaptureRootV1,
    row: &CheckedRetainedResultRowV1<'_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    attached.require_original_grant(grant)?;
    let original = row.subject().map_err(ownership)?;
    let subject = attached.subject();
    grant.charge_steps(1).map_err(domain)?;
    let locator = super::objects::reference(original.locator().as_bytes(), grant)?;
    if subject.role != AttachmentSubjectRoleV1::ResultRow
        || subject.capture != root.payload.digest
        || subject.body.as_str() != original.body_sha256()
        || subject.interpretation.as_str() != row.reference()
        || subject.locator != locator.digest
        || subject.subject.as_str() != original.reference()
        || subject.context.as_str() != row.context_reference()
        || subject.namespace.as_str() != original.document_namespace()
        || subject.native_id.as_ref().map(|value| value.as_str())
            != Some(original.native_document_id())
    {
        return Err(invalid(
            "school quote differs from accepted original person row",
        ));
    }
    Ok(())
}
