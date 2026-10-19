use super::{guarded, invalid};
use crate::participation_evidence::recorded::{
    CheckedDocumentaryParticipationFactsV1, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    DiscoveredResultAttachmentV1,
};
use crate::participation_evidence::result_ownership::read_retained_html_result_rows;
use crate::participation_evidence::{ParticipationEvidenceRefV1, RetainedHtmlCaptureV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_calendar_correspondence::ResultCalendarReviewedRecordV1;
use census_store::{ReplayedResultCalendarCorrespondencesV1, RetainedCaptureRootSchemaV1};

pub struct CheckedResultCalendarCorrespondenceV1<'a> {
    record: &'a ResultCalendarReviewedRecordV1,
    documentary_reference: &'a ParticipationEvidenceRefV1,
}

impl<'a> CheckedResultCalendarCorrespondenceV1<'a> {
    pub fn record(&self) -> &'a ResultCalendarReviewedRecordV1 {
        self.record
    }

    pub fn documentary_reference(&self) -> &'a ParticipationEvidenceRefV1 {
        self.documentary_reference
    }
}

pub fn checked_frozen<'a>(
    source: &'a ReplayedResultCalendarCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedResultCalendarCorrespondenceV1<'a>>> {
    guarded(grant, |grant| check(source, linked, index, facts, grant))
}

fn check<'a>(
    source: &'a ReplayedResultCalendarCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<CheckedResultCalendarCorrespondenceV1<'a>>> {
    source.require_original_grant(grant)?;
    let linked_row = linked
        .row(index)
        .ok_or_else(|| invalid("checked calendar row absent"))?;
    let attached = linked_row.attachment();
    attached.require_original_grant(grant)?;
    require_result_owner(source, attached)?;
    let attachments = source.attachments(grant)?;
    if attachments.store_identity() != attached.store_identity()
        || attachments.snapshot_sequence() != attached.snapshot_sequence()
        || attachments.accepted_public_head() != attached.accepted_public_head()
        || attachments.inventory() != attached.inventory()
    {
        return Err(invalid("frozen calendar attachment owner differs"));
    }
    let actual = attachments
        .bind_exact_subject(attached.subject(), grant)?
        .ok_or_else(|| invalid("frozen calendar active person absent"))?;
    actual.require_original_grant(grant)?;
    if actual.record() != attached.record()
        || actual.target() != attached.target()
        || actual.receipt() != attached.receipt()
    {
        return Err(invalid("frozen calendar active person differs"));
    }
    let selected = select(source, linked, attached, facts, grant)?;
    let Some((record, documentary_reference)) = selected else {
        return Ok(None);
    };
    require_original(
        source,
        linked,
        linked_row.checked_row(),
        attached,
        record,
        grant,
    )?;
    Ok(Some(CheckedResultCalendarCorrespondenceV1 {
        record,
        documentary_reference,
    }))
}

fn require_result_owner(
    source: &ReplayedResultCalendarCorrespondencesV1<'_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
) -> CrawlResult<()> {
    if source.store_identity() != attached.store_identity()
        || source.snapshot_sequence() != attached.snapshot_sequence()
    {
        return Err(invalid("frozen calendar result owner differs"));
    }
    Ok(())
}

fn select<'a>(
    authority: &'a ReplayedResultCalendarCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<
    Option<(
        &'a ResultCalendarReviewedRecordV1,
        &'a ParticipationEvidenceRefV1,
    )>,
> {
    let mut found = None;
    for record in authority.active_records(grant)? {
        grant.charge_steps(1).map_err(super::domain)?;
        let source = &record.command.source;
        if source.person != *attached.target()
            || source.result_attachment.operation != attached.record().command.operation
            || source.result_attachment.digest != attached.record().record_digest
            || source.result.body.digest != attached.subject().body
            || source.result.interpretation.digest != attached.subject().interpretation
            || source.result.row_locator != attached.subject().locator
        {
            continue;
        }
        let (result_root, documentary_root) =
            authority.reviewed_source_roots(&record.record_digest, grant)?;
        if &result_root != linked.recorded_root() {
            continue;
        }
        let Some(documentary_reference) =
            documentary_matches(facts, record, &documentary_root, grant)?
        else {
            continue;
        };
        if found.is_some() {
            return Err(invalid("ambiguous active calendar correspondence"));
        }
        found = Some((record, documentary_reference));
    }
    Ok(found)
}

fn documentary_matches<'a>(
    facts: &'a CheckedDocumentaryParticipationFactsV1<'a>,
    record: &ResultCalendarReviewedRecordV1,
    root: &census_store::RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<&'a ParticipationEvidenceRefV1>> {
    let accepted = &record.command.source;
    let mut found = None;
    for fact in facts.sources() {
        grant.charge_steps(1).map_err(super::domain)?;
        let source = fact.source();
        if fact.attachment_record_digest() != accepted.documentary_attachment.digest.as_str()
            || source.day() != &accepted.day
            || source.school_year() != accepted.year
            || source.season_proof().capture() != accepted.calendar.capture.as_str()
            || source.season_proof().interpretation() != accepted.calendar.interpretation.as_str()
            || source.season_scope() != accepted.calendar.scope.as_str()
        {
            continue;
        }
        if super::objects::digest(source.season_proof().locator().as_bytes(), grant)?
            != accepted.calendar.locator
        {
            continue;
        }
        if fact.root() != root {
            continue;
        }
        if found.is_some() {
            return Err(invalid("ambiguous documentary calendar fact"));
        }
        found = Some(source.reference());
    }
    Ok(found)
}

fn require_original(
    source: &ReplayedResultCalendarCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    linked_row: &crate::participation_evidence::result_ownership::CheckedRetainedResultRowV1<'_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    record: &ResultCalendarReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let original = source
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            &linked.recorded_root().key,
            grant,
        )?
        .ok_or_else(|| invalid("frozen calendar original result absent"))?;
    if original.root() != linked.recorded_root() {
        return Err(invalid("frozen calendar original result root differs"));
    }
    let capture: RetainedHtmlCaptureV1 = serde_json::from_str(original.payload().get())
        .map_err(crate::participation_evidence::json)?;
    let document = read_retained_html_result_rows(&capture, &[linked_row.locator()])
        .map_err(super::super::original::ownership)?;
    let row = document
        .row(0)
        .ok_or_else(|| invalid("frozen calendar original selected row absent"))?;
    super::super::original::subject(original.root(), &row, attached, grant)?;
    let scope = super::scope(&document, &row)?;
    let accepted = &record.command.source;
    if accepted.day != scope.day
        || accepted.sport != scope.sport
        || accepted.category != scope.category
        || accepted.year != scope.year
    {
        return Err(invalid("frozen calendar result date linkage differs"));
    }
    let mut objects = super::objects::Objects::new(grant)?;
    let result = super::result_claim(original.root(), &document, &row, &mut objects, grant)?;
    if accepted.result != result {
        return Err(invalid(
            "frozen calendar result differs from original source",
        ));
    }
    Ok(())
}
