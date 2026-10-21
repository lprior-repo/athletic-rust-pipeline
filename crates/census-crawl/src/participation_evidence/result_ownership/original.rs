use crate::participation_evidence::recorded::{
    read_recorded_source_html_result_capture, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    DiscoveredResultAttachmentV1, RecordedHtmlResultCaptureV1,
};
use crate::participation_evidence::result_ownership::CheckedHtmlResultRowV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    AttachmentSubjectRoleV1,
};
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, RetainedCaptureRootV1, StoreSnapshot,
};
use sha2::{Digest, Sha256};

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
    row: &CheckedHtmlResultRowV1<'_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    attached.require_original_grant(grant)?;
    let original = row.subject().map_err(ownership)?;
    let subject = attached.subject();
    grant.charge_steps(1).map_err(domain)?;
    let locator = reference(original.locator().as_bytes(), grant)?;
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
pub(super) fn reopen(
    snapshot: &StoreSnapshot<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    row: &CheckedHtmlResultRowV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RecordedHtmlResultCaptureV1> {
    let recorded = read_recorded_source_html_result_capture(
        snapshot,
        &linked.recorded_root().key,
        &[row.locator()],
        row.source(),
        grant,
    )?
    .ok_or_else(|| invalid("school original capture absent"))?;
    if recorded.root() != linked.recorded_root() {
        return Err(invalid("school original capture root differs"));
    }
    Ok(recorded)
}
pub(super) fn invalid(detail: &'static str) -> crate::CrawlError {
    crate::participation_evidence::invalid(detail).into()
}

pub(super) fn domain(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> crate::CrawlError {
    census_store::StoreError::from(census_store::EvidenceSubjectAttachmentStoreError::Domain(
        error,
    ))
    .into()
}

pub(super) fn ownership(error: super::RetainedResultOwnershipError) -> crate::CrawlError {
    match error {
        super::RetainedResultOwnershipError::Parser(error) => error,
        super::RetainedResultOwnershipError::Capture(error) => error.into(),
        super::RetainedResultOwnershipError::Digest(source) => crate::CrawlError::Canonical {
            table: "result school original row".into(),
            source,
        },
        super::RetainedResultOwnershipError::Invalid(detail) => invalid(detail),
        super::RetainedResultOwnershipError::Overflow => {
            census_store::StoreError::CounterOverflow.into()
        }
        super::RetainedResultOwnershipError::Allocation => domain(
            census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Allocation,
        ),
        super::RetainedResultOwnershipError::Capacity { needed, available } => {
            crate::CrawlError::Resource {
                resource: "result school original rows",
                requested: needed,
                limit: available,
            }
        }
    }
}

pub(super) fn guarded<T>(
    grant: &mut AttachmentReplayGrantV1,
    action: impl FnOnce(&mut AttachmentReplayGrantV1) -> CrawlResult<T>,
) -> CrawlResult<T> {
    grant.ensure_available().map_err(domain)?;
    let result = action(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}
pub(super) fn reference(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentObjectReferenceV1> {
    grant.charge_bytes(bytes.len()).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))
            .map_err(domain)?,
        bytes: u64::try_from(bytes.len()).map_err(|_| census_store::StoreError::CounterOverflow)?,
    })
}
