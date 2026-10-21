use super::{read, RecordedHtmlResultCaptureV1};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    read_retained_source_result_rows, CheckedHtmlResultDocumentV1, CheckedHtmlResultSourceV1,
};
use crate::{CrawlError, CrawlResult};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, EvidenceSubjectAttachmentStoreError, StoreError,
};

mod discovery;
mod event_label;
mod subject;
mod types;
mod venue;
pub(super) mod view;

pub use discovery::{
    discover_current_html_results, discover_frozen_html_results,
    AttachmentHtmlResultDiscoveryGapV1, AttachmentHtmlResultDiscoveryV1,
    UnresolvedAttachmentHtmlResultV1,
};
pub use types::{
    AttachedResultGapV1, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    DiscoveredIdentityLinkedHtmlResultRowV1, DiscoveredResultAttachmentV1,
    IdentityLinkedHtmlResultDocumentV1, IdentityLinkedHtmlResultRowV1,
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
    with_results(
        view::View::Current(current),
        context_reference,
        requested,
        CheckedHtmlResultSourceV1::MileSplit,
        grant,
        |linked, grant| {
            let view::Bindings::Current(attachments) = linked.attachments else {
                return Err(super::super::invalid("live result bindings differ").into());
            };
            let CheckedHtmlResultDocumentV1::MileSplit(document) = linked.document else {
                return Err(
                    super::super::invalid("explicit MileSplit result source differs").into(),
                );
            };
            consume(
                IdentityLinkedHtmlResultDocumentV1 {
                    document,
                    root: linked.root,
                    snapshot: current.snapshot(),
                    attachments,
                },
                grant,
            )
        },
    )
}

fn with_results<'s, T>(
    current: view::View<'_, '_, 's>,
    context_reference: &str,
    requested: &[SourceRowLocator],
    expected_source: CheckedHtmlResultSourceV1,
    grant: &mut AttachmentReplayGrantV1,
    consume: impl for<'a> FnOnce(
        DiscoveredIdentityLinkedHtmlResultDocumentV1<'a, 's>,
        &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<T>,
) -> CrawlResult<Option<T>> {
    read::with_grant(grant, |grant| {
        grant.charge_steps(1).map_err(domain)?;
        let recorded = match expected_source {
            CheckedHtmlResultSourceV1::MileSplit => read::read_html_result_capture_from(
                current.source(),
                context_reference,
                requested,
                grant,
            )?,
            CheckedHtmlResultSourceV1::Tfrrs => {
                read::reopen_html_result_capture(current.source(), context_reference, grant)?
            }
        };
        let Some(recorded) = recorded else {
            return Ok(None);
        };
        grant.charge_steps(1).map_err(domain)?;
        let document =
            read_retained_source_result_rows(recorded.capture(), requested, expected_source)
                .map_err(read::ownership)?;
        if document.context_reference() != recorded.root().key {
            return Err(
                super::super::invalid("source result context differs from retained root").into(),
            );
        }
        let attachments = current.bindings(&recorded, &document, grant)?;
        let linked = DiscoveredIdentityLinkedHtmlResultDocumentV1 {
            document: &document,
            root: recorded.root(),
            attachments,
            venue: std::sync::OnceLock::new(),
        };
        consume(linked, grant).map(Some)
    })
}

fn domain(error: EvidenceSubjectAttachmentError) -> CrawlError {
    StoreError::from(EvidenceSubjectAttachmentStoreError::Domain(error)).into()
}

#[cfg(test)]
pub(in crate::participation_evidence) use discovery::tests::tfrrs::install_controlled_tfrrs_result_attachments;
#[cfg(test)]
pub(in crate::participation_evidence::recorded) use discovery::tests::verify_genuine as verify_discovery_genuine;
