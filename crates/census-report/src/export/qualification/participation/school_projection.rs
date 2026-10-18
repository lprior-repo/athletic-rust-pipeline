use crate::grouping::Admission;
use crate::report::{documentary_source_error as source_error, ReportResult};
use census_crawl::participation_evidence::physical_result::{
    project_retained_html_result_row, ProposedHtmlResultTargetV1,
};
use census_crawl::participation_evidence::recorded::{
    discover_frozen_html_results, DiscoveredIdentityLinkedHtmlResultDocumentV1,
};
use census_crawl::participation_evidence::result_ownership::school_correspondence::read::checked_frozen;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, PerformanceId,
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
    let projection = project_retained_html_result_row(linked.checked_document(), index, &target)
        .map_err(source_error)?;
    let (meet, proposed_rows) = projection.into_proposed_entities();
    if proposed_rows.len() != 1 {
        return Err(invalid("single row projection returned unexpected row count").into());
    }
    let first = match proposed_rows.into_iter().next() {
        Some(value) => value,
        None => return Err(invalid("projection row vanished").into()),
    };
    let (event, team, performance) = first.into_proposed_entities();
    Ok(Some(SchoolProjectedRow {
        meet,
        event,
        team,
        performance,
    }))
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
