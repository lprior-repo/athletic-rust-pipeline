use super::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use super::{domain, grouping, invalid, read, reserve, view, Member};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    read_retained_source_result_spans, CheckedHtmlResultDocumentV1, CheckedHtmlResultSourceV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::ActiveAttachmentSubjectDiscoveryV1;

pub(super) fn group<'s, T>(
    view: view::View<'_, '_, 's>,
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    members: &[Member],
    grant: &mut AttachmentReplayGrantV1,
    consume: impl for<'r> FnOnce(
        DiscoveredIdentityLinkedHtmlResultDocumentV1<'r, 's>,
        &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<T>,
) -> CrawlResult<T> {
    let first = members
        .first()
        .ok_or_else(|| invalid("empty result group"))?;
    let (root, _) = grouping::key(metadata, first.index)?;
    let expected_source = source(metadata, first.index)?;
    let locators = recover(view, metadata, members, expected_source, grant)?;
    super::super::with_results(view, &root.key, &locators, expected_source, grant, consume)?
        .ok_or_else(|| invalid("discovered HTML result capture absent").into())
}

fn recover(
    view: view::View<'_, '_, '_>,
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    members: &[Member],
    expected_source: CheckedHtmlResultSourceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<SourceRowLocator>> {
    let first = members
        .first()
        .ok_or_else(|| invalid("empty result group"))?;
    let (root, _) = grouping::key(metadata, first.index)?;
    let recorded = read::reopen_html_result_capture(view.source(), &root.key, grant)?
        .ok_or_else(|| invalid("discovered HTML result capture absent"))?;
    grant.charge_steps(1).map_err(domain)?;
    if recorded.root() != root {
        return Err(invalid("discovered HTML capture descriptor differs").into());
    }
    let mut spans = reserve(members.len(), grant)?;
    for member in members {
        grant.charge_steps(1).map_err(domain)?;
        spans.push(member.span);
    }
    grant.charge_steps(1).map_err(domain)?;
    let document =
        read_retained_source_result_spans(recorded.capture(), &spans, expected_source, grant)
            .map_err(read::ownership)?;
    grant.charge_steps(1).map_err(domain)?;
    if document.context_reference() != root.key {
        return Err(invalid("recorded HTML result context differs from its typed key").into());
    }
    locators(&recorded, &document, metadata, members, grant)
}

fn locators(
    recorded: &super::super::RecordedHtmlResultCaptureV1,
    document: &CheckedHtmlResultDocumentV1<'_>,
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    members: &[Member],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<SourceRowLocator>> {
    let mut locators = reserve(members.len(), grant)?;
    for (index, member) in members.iter().enumerate() {
        grant.charge_steps(1).map_err(domain)?;
        let row = document
            .row(index)
            .ok_or_else(|| invalid("discovered result row absent"))?;
        let expected = metadata
            .subjects()
            .get(member.index)
            .ok_or_else(|| invalid("discovery subject absent"))?;
        let actual = super::super::subject::reference(recorded, &row, grant)?;
        grant.charge_steps(1).map_err(domain)?;
        if !member.span.matches(row.locator()) || &actual != expected.subject() {
            return Err(invalid("discovered original result subject differs").into());
        }
        locators.push(row.locator());
    }
    Ok(locators)
}

fn source(
    metadata: &ActiveAttachmentSubjectDiscoveryV1<'_>,
    index: usize,
) -> CrawlResult<CheckedHtmlResultSourceV1> {
    let subject = metadata
        .subjects()
        .get(index)
        .ok_or_else(|| invalid("discovery subject absent"))?;
    CheckedHtmlResultSourceV1::from_document_namespace(subject.subject().namespace.as_str())
        .ok_or_else(|| invalid("unsupported discovered result namespace").into())
}
