use super::super::result_ownership::CheckedRetainedResultDocumentV1;
use super::{read, read_recorded_html_result_capture, RecordedHtmlResultCaptureV1};
use crate::milesplit::SourceRowLocator;
use crate::{CrawlError, CrawlResult};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, CurrentEvidenceSubjectAttachmentV1,
    EvidenceSubjectAttachmentStoreError, StoreError,
};

mod subject;
mod types;

pub use types::{
    AttachedResultGapV1, IdentityLinkedHtmlResultDocumentV1, IdentityLinkedHtmlResultRowV1,
};

pub fn with_attached_html_results<'s, T>(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
    context_reference: &str,
    requested: &[SourceRowLocator],
    grant: &mut AttachmentReplayGrantV1,
    consume: impl for<'a> FnOnce(
        IdentityLinkedHtmlResultDocumentV1<'a, 's>,
        &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<T>,
) -> CrawlResult<Option<T>> {
    read::with_grant(grant, |grant| {
        grant.charge_steps(1).map_err(domain)?;
        let Some(recorded) = read_recorded_html_result_capture(
            current.snapshot(),
            context_reference,
            requested,
            grant,
        )?
        else {
            return Ok(None);
        };
        grant.charge_steps(1).map_err(domain)?;
        let document = recorded.read(requested)?;
        let attachments = bindings(current, &recorded, &document, grant)?;
        let linked = IdentityLinkedHtmlResultDocumentV1 {
            document: &document,
            root: recorded.root(),
            snapshot: current.snapshot(),
            attachments,
        };
        consume(linked, grant).map(Some)
    })
}

fn bindings<'a, 's>(
    current: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
    recorded: &RecordedHtmlResultCaptureV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<CurrentEvidenceSubjectAttachmentV1<'a, 's>>> {
    let count = document.selected_count();
    grant.charge_steps(count).map_err(domain)?;
    let mut attachments = Vec::new();
    attachments
        .try_reserve_exact(count)
        .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))?;
    for index in 0..count {
        let row = document
            .row(index)
            .ok_or_else(|| super::super::invalid("checked attached result row absent"))?;
        let expected = subject::reference(recorded, &row, grant)?;
        let attachment = current
            .bind_exact_subject(&expected, grant)?
            .ok_or_else(|| {
                super::super::invalid("original result row has no active current attachment")
            })?;
        attachment.require_original_grant(grant)?;
        attachments.push(attachment);
    }
    Ok(attachments)
}

fn domain(error: EvidenceSubjectAttachmentError) -> CrawlError {
    StoreError::from(EvidenceSubjectAttachmentStoreError::Domain(error)).into()
}
