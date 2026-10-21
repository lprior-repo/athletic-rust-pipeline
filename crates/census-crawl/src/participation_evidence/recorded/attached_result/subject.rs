use super::{domain, RecordedHtmlResultCaptureV1};
use crate::participation_evidence::result_ownership::CheckedHtmlResultRowV1;
use crate::participation_evidence::{hash, DocumentSubjectV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentReplayGrantV1, AttachmentSubjectRoleV1, AttachmentTextV1,
    RetainedDocumentSubjectReferenceV1,
};
use census_store::StoreError;

pub(super) fn reference(
    recorded: &RecordedHtmlResultCaptureV1,
    row: &CheckedHtmlResultRowV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedDocumentSubjectReferenceV1> {
    grant.charge_steps(10).map_err(domain)?;
    let source = row.subject().map_err(super::super::read::ownership)?;
    grant
        .charge_bytes(reference_owner_bytes(&source)?)
        .map_err(domain)?;
    grant
        .charge_bytes(preimage_bytes(&source)?)
        .map_err(domain)?;
    grant.charge_bytes(64).map_err(domain)?;
    let subject = source.reference();
    grant.charge_bytes(source.locator().len()).map_err(domain)?;
    grant.charge_bytes(64).map_err(domain)?;
    let locator = hash(source.locator().as_bytes());
    Ok(RetainedDocumentSubjectReferenceV1 {
        namespace: AttachmentTextV1::parse(source.document_namespace()).map_err(domain)?,
        native_id: Some(AttachmentTextV1::parse(source.native_document_id()).map_err(domain)?),
        capture: recorded.root().payload.digest.clone(),
        body: digest(source.body_sha256())?,
        interpretation: digest(row.reference())?,
        locator: digest(&locator)?,
        subject: digest(&subject)?,
        context: digest(row.context_reference())?,
        role: AttachmentSubjectRoleV1::ResultRow,
    })
}

fn reference_owner_bytes(subject: &DocumentSubjectV1) -> Result<usize, StoreError> {
    [subject.document_namespace(), subject.native_document_id()]
        .iter()
        .try_fold(6usize * 64, |bytes, field| {
            bytes
                .checked_add(field.len())
                .ok_or(StoreError::CounterOverflow)
        })
}

fn preimage_bytes(subject: &DocumentSubjectV1) -> Result<usize, StoreError> {
    [
        "document_subject_v1",
        subject.document_namespace(),
        subject.native_document_id(),
        subject.document_url(),
        subject.body_sha256(),
        subject.locator(),
    ]
    .iter()
    .try_fold(5usize, |bytes, field| {
        bytes
            .checked_add(field.len())
            .ok_or(StoreError::CounterOverflow)
    })
}

fn digest(value: &str) -> CrawlResult<AttachmentDigestV1> {
    AttachmentDigestV1::parse(value).map_err(domain)
}
