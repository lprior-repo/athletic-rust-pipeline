use super::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use super::{domain, grouping, invalid, read, reserve, view, Member};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::discovery as source_rows;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::ActiveResultSubjectDiscoveryV1;

pub(super) fn group<'s, T>(
    view: view::View<'_, '_, 's>,
    metadata: &ActiveResultSubjectDiscoveryV1<'_>,
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
    let (_, context) = grouping::key(metadata, first.index)?;
    let locators = recover(view, metadata, members, grant)?;
    super::super::with_results(view, context.digest.as_str(), &locators, grant, consume)?
        .ok_or_else(|| invalid("discovered HTML result capture absent").into())
}

fn recover(
    view: view::View<'_, '_, '_>,
    metadata: &ActiveResultSubjectDiscoveryV1<'_>,
    members: &[Member],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<SourceRowLocator>> {
    let first = members
        .first()
        .ok_or_else(|| invalid("empty result group"))?;
    let (root, context) = grouping::key(metadata, first.index)?;
    let recorded = read::reopen_html_result_capture(view.source(), context.digest.as_str(), grant)?
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
    let document = source_rows::read(recorded.capture(), &spans, grant).map_err(read::ownership)?;
    grant.charge_steps(1).map_err(domain)?;
    if document.context_reference() != context.digest.as_str() {
        return Err(invalid("recorded HTML result context differs from its typed key").into());
    }
    locators(&recorded, &document, metadata, members, grant)
}

fn locators(
    recorded: &super::super::RecordedHtmlResultCaptureV1,
    document: &crate::participation_evidence::result_ownership::CheckedRetainedResultDocumentV1<'_>,
    metadata: &ActiveResultSubjectDiscoveryV1<'_>,
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
