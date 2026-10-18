use super::super::{guarded, invalid};
use super::{select, source, CheckedResultSchoolCorrespondenceV1};
use crate::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::ReplayedResultSchoolCorrespondencesV1;

pub fn checked_frozen<'a>(
    current: &'a ReplayedResultSchoolCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedResultSchoolCorrespondenceV1<'a>>> {
    guarded(grant, |grant| check(current, linked, index, grant))
}

fn check<'a>(
    current: &'a ReplayedResultSchoolCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedResultSchoolCorrespondenceV1<'a>>> {
    current.require_original_grant(grant)?;
    let linked_row = linked
        .row(index)
        .ok_or_else(|| invalid("frozen school row absent"))?;
    let attached = linked_row.attachment();
    attached.require_original_grant(grant)?;
    let attachments = current.attachments(grant)?;
    if current.store_identity() != attached.store_identity()
        || current.snapshot_sequence() != attached.snapshot_sequence()
        || attachments.accepted_public_head() != attached.accepted_public_head()
        || attachments.inventory() != attached.inventory()
    {
        return Err(invalid("frozen school attachment owner differs"));
    }
    let actual = attachments
        .bind_exact_subject(attached.subject(), grant)?
        .ok_or_else(|| invalid("frozen school active person absent"))?;
    if actual.record() != attached.record() || actual.target() != attached.target() {
        return Err(invalid("frozen school active person differs"));
    }
    let Some(record) = select(current.active_records(grant)?, attached)? else {
        return Ok(None);
    };
    source::require(
        source::Source::Frozen(current),
        linked,
        index,
        record,
        grant,
    )?;
    Ok(Some(CheckedResultSchoolCorrespondenceV1 { record }))
}
