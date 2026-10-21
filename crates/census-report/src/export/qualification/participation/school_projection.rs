use crate::grouping::Admission;
use crate::report::{documentary_source_error as source_error, ReportResult};
use census_crawl::participation_evidence::physical_result::{
    project_checked_html_result_row, ProposedHtmlResultTargetV1,
};
use census_crawl::participation_evidence::provider_target::CheckedLinkedEventLabelV1;
use census_crawl::participation_evidence::recorded::{
    discover_frozen_html_results, DiscoveredIdentityLinkedHtmlResultDocumentV1,
};
use census_crawl::participation_evidence::result_ownership::school_correspondence::read::checked_frozen;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentFactDispositionV1, AttachmentReplayGrantV1,
};
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventDisposition,
    EventResolutionReason, EventSemantics, Evidence, PerformanceId, RecruitingCatalogV1,
    SourceEventLabel, Sport,
};
use census_store::ReplayedResultSchoolCorrespondencesV1;
use std::collections::BTreeSet;

pub(crate) struct SchoolProjection {
    pub(crate) rows: Vec<SchoolProjectedRow>,
}

#[derive(serde::Serialize)]
pub(crate) struct SchoolProjectedRow {
    pub(crate) meet: CanonicalMeet,
    pub(crate) event: CanonicalEvent,
    pub(crate) team: CanonicalTeam,
    pub(crate) performance: CanonicalPerformance,
    #[serde(skip)]
    pub(crate) original_event_label: Option<SourceEventLabel>,
}

pub(crate) fn collect(
    source: &ReplayedResultSchoolCorrespondencesV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<SchoolProjection> {
    let attachments = source.attachments(grant)?;
    let plan = discover_frozen_html_results(attachments, grant).map_err(source_error)?;
    let mut budget = Admission::new("school projected physical performances");
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for group_index in 0..plan.group_count() {
        let result = plan
            .with_group(group_index, grant, |linked, grant| {
                Ok((|| -> ReportResult<()> {
                    for row_index in 0..linked.row_count() {
                        let row = project_row(source, &linked, row_index, grant)?;
                        if let Some(projected) = row {
                            admit_row(projected, &mut seen, &mut budget, &mut rows)?;
                        }
                    }
                    Ok(())
                })())
            })
            .map_err(source_error)?;
        result?;
    }
    Ok(SchoolProjection { rows })
}

fn project_row(
    source: &ReplayedResultSchoolCorrespondencesV1<'_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<Option<SchoolProjectedRow>> {
    let linked_row = linked
        .row(index)
        .ok_or_else(|| invalid("frozen school result row absent"))?;
    let attachment = linked_row.attachment();
    attachment.require_original_grant(grant)?;
    let checked = checked_frozen(source, linked, index, grant).map_err(source_error)?;
    let checked = match checked {
        Some(value) => value,
        None => return Ok(None),
    };
    let school = checked.school();
    let candidate = attachment.target().canonical_candidate.cast();
    let subject = linked_row
        .checked_row()
        .subject()
        .map_err(
            census_crawl::participation_evidence::physical_result::HtmlResultProjectionError::from,
        )
        .map_err(source_error)?;
    let subject_reference = subject.reference();
    let target = ProposedHtmlResultTargetV1 {
        locator: linked_row.checked_row().locator(),
        subject_reference: &subject_reference,
        proposed_athlete: &candidate,
        proposed_school: school,
        proposed_attachment_reference: attachment.record().record_digest.as_str(),
        proposed_school_binding_reference: checked.record().record_digest.as_str(),
    };
    let (meet, first) = project_checked_html_result_row(linked.checked_document(), index, &target)
        .map_err(source_error)?;
    let (mut event, team, performance) = first.into_proposed_entities();
    let original_event_label = reviewed_event_context(linked, index, &mut event, grant)?;
    Ok(Some(SchoolProjectedRow {
        meet,
        event,
        team,
        performance,
        original_event_label,
    }))
}

fn reviewed_event_context(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    event: &mut CanonicalEvent,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<Option<SourceEventLabel>> {
    let row = linked
        .row(index)
        .ok_or_else(|| invalid("reviewed event context result row absent"))?;
    let attachment = row.attachment();
    let facts = attachment
        .record()
        .command
        .reviewed_context
        .facts
        .as_slice();
    let subject = &attachment.subject().subject;
    grant
        .charge_steps(facts.len())
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)
        .map_err(census_store::StoreError::from)?;
    if !facts.iter().any(|fact| {
        &fact.subject == subject && fact.disposition == AttachmentFactDispositionV1::ContextOnly
    }) {
        return Ok(None);
    }
    if RecruitingCatalogV1::with_policy(EventSemantics::CURRENT)
        .classify(event, Sport::Unknown)
        .map_err(|source| crate::report::ReportError::Catalog {
            event: event.id.clone(),
            source,
        })?
        != EventDisposition::Unresolved(EventResolutionReason::UnknownOrMixedSourceLabels)
    {
        return Ok(None);
    }
    let Some(label) = linked
        .checked_event_label(index, grant)
        .map_err(source_error)?
    else {
        return Ok(None);
    };
    grant
        .charge_steps(facts.len())
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)
        .map_err(census_store::StoreError::from)?;
    if !facts.iter().any(|fact| {
        &fact.subject == subject
            && fact.fact == label.heading_reference().digest
            && fact.disposition == AttachmentFactDispositionV1::ContextOnly
    }) {
        return Ok(None);
    }
    apply_reviewed_event_context(event, label, &attachment.record().record_digest)
}

fn apply_reviewed_event_context(
    event: &mut CanonicalEvent,
    label: CheckedLinkedEventLabelV1,
    review: &AttachmentDigestV1,
) -> ReportResult<Option<SourceEventLabel>> {
    if event.source_labels.len() != 1 {
        return Err(invalid("checked event context has ambiguous original source labels").into());
    }
    let original = event
        .source_labels
        .pop()
        .ok_or_else(|| invalid("checked event context original source label absent"))?;
    let note = EventContextNote {
        role: "checked_linked_result_event_context",
        original: &original,
        heading: label.heading_reference(),
        capture: label.heading_proof().capture(),
        locator: label.heading_proof().locator(),
        row: label.row_reference(),
        review,
    };
    let encoded =
        serde_json::to_string(&note).map_err(|source| crate::report::ReportError::Invariant {
            detail: format!("encoding checked source event context: {source}"),
        })?;
    let observed = label.heading_proof().acquired_at().to_rfc3339();
    let source_label = label.into_source_label();
    let mut evidence = Evidence::parsed(source_label.source.clone(), observed);
    evidence.note = Some(encoded);
    event
        .evidence
        .try_reserve(1)
        .map_err(|_| invalid("checked event context evidence allocation refused"))?;
    event.source_labels.push(source_label);
    event.evidence.push(evidence);
    Ok(Some(original))
}

#[derive(serde::Serialize)]
struct EventContextNote<'a> {
    role: &'static str,
    original: &'a SourceEventLabel,
    heading: &'a census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1,
    capture: &'a str,
    locator: &'a str,
    row: &'a census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1,
    review: &'a census_domain::evidence_subject_attachment::AttachmentDigestV1,
}

fn admit_row(
    row: SchoolProjectedRow,
    seen: &mut BTreeSet<PerformanceId>,
    budget: &mut Admission,
    rows: &mut Vec<SchoolProjectedRow>,
) -> ReportResult<()> {
    budget.record(&row)?;
    rows.try_reserve(1)
        .map_err(|_| invalid("school projection allocation refused"))?;
    if !seen.insert(row.performance.id.clone()) {
        return Err(invalid("school projection contains duplicate performance identity").into());
    }
    rows.push(row);
    Ok(())
}

fn invalid(detail: &str) -> census_store::StoreError {
    super::invalid(detail)
}
