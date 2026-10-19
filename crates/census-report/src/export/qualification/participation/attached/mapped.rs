use crate::export::ExportDataset;
use crate::grouping::Admission;
use crate::report::{documentary_source_error as source, ReportResult};
use census_crawl::participation_evidence::physical_result::{
    project_retained_html_result_row, ProposedHtmlResultTargetV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_domain::model::PerformanceId;
use census_domain::participation_candidates::CandidateQuery;
use std::collections::BTreeMap;

pub(super) fn insert(
    dataset: &ExportDataset,
    query: CandidateQuery<'_>,
    sources: &census_crawl::participation_evidence::recorded::AttachedParticipationV1<'_, '_>,
    calendar: Option<&census_store::ReplayedResultCalendarCorrespondencesV1<'_>>,
    linked: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1<
        '_,
        '_,
    >,
    index: usize,
    checked: &census_crawl::participation_evidence::result_ownership::school_correspondence::read::CheckedResultSchoolCorrespondenceV1<'_>,
    grant: &mut Grant,
    budget: &mut Admission,
    rows: &BTreeMap<PerformanceId, crate::export::qualification::DocumentaryParticipationRow>,
    mapped: &mut BTreeMap<PerformanceId, super::super::PinnedParticipationRow>,
) -> ReportResult<()> {
    let assessed = super::facts::assess(sources, calendar, linked, index, query, grant)?;
    let row = linked
        .row(index)
        .ok_or_else(|| super::super::invalid("completed documentary row is missing"))?;
    let projected = project(linked, index, &row, checked, grant, budget)?;
    super::validate::canonical(
        dataset,
        &projected.0,
        &projected.1,
        &projected.2,
        &projected.3,
        checked.school(),
        budget,
    )?;
    let references = references(&row, assessed.2, checked, budget)?;
    super::shared::admit_pinned(budget, &projected.3, &references)?;
    super::reject_mapped_duplicate(rows, mapped, &projected.3.id, budget)?;
    let search = rows
        .len()
        .checked_add(mapped.len())
        .and_then(|count| count.checked_add(1))
        .ok_or(census_store::StoreError::CounterOverflow)?;
    let owner = std::mem::size_of::<(PerformanceId, super::super::PinnedParticipationRow)>()
        .checked_add(projected.3.id.as_str().len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    budget.charge(owner, search)?;
    mapped.insert(
        projected.3.id.clone(),
        super::super::PinnedParticipationRow {
            outcome: assessed.0,
            reasons: assessed.1,
            references,
            diagnostics: Vec::new(),
        },
    );
    Ok(())
}

fn project(
    linked: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1<
        '_,
        '_,
    >,
    index: usize,
    row: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<
        '_,
        '_,
    >,
    checked: &census_crawl::participation_evidence::result_ownership::school_correspondence::read::CheckedResultSchoolCorrespondenceV1<'_>,
    grant: &mut Grant,
    budget: &mut Admission,
) -> ReportResult<(
    census_domain::model::CanonicalMeet,
    census_domain::model::CanonicalEvent,
    census_domain::model::CanonicalTeam,
    census_domain::model::CanonicalPerformance,
)> {
    let attachment = row.attachment();
    attachment.require_original_grant(grant)?;
    super::step(grant, 1)?;
    budget.charge(attachment.target().canonical_candidate.as_str().len(), 1)?;
    let candidate = attachment.target().canonical_candidate.cast();
    let subject = row
        .checked_row()
        .subject()
        .map_err(
            census_crawl::participation_evidence::physical_result::HtmlResultProjectionError::from,
        )
        .map_err(source)?;
    let subject_reference = subject.reference();
    let target = ProposedHtmlResultTargetV1 {
        locator: row.checked_row().locator(),
        subject_reference: subject_reference.as_str(),
        proposed_athlete: &candidate,
        proposed_school: checked.school(),
        proposed_attachment_reference: attachment.record().record_digest.as_str(),
        proposed_school_binding_reference: checked.record().record_digest.as_str(),
    };
    let projection = project_retained_html_result_row(linked.checked_document(), index, &target)
        .map_err(source)?;
    let (meet, proposed) = projection.into_proposed_entities();
    if proposed.len() != 1 {
        return Err(
            super::super::invalid("single row projection returned unexpected row count").into(),
        );
    }
    let first = match proposed.into_iter().next() {
        Some(value) => value,
        None => {
            return Err(super::super::invalid("projection row vanished").into());
        }
    };
    let (event, team, performance) = first.into_proposed_entities();
    Ok((meet, event, team, performance))
}

fn references(
    row: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<
        '_,
        '_,
    >,
    semantic: Vec<String>,
    checked: &census_crawl::participation_evidence::result_ownership::school_correspondence::read::CheckedResultSchoolCorrespondenceV1<'_>,
    budget: &mut Admission,
) -> ReportResult<Vec<String>> {
    let base = super::shared::base_refs(row);
    let mut owned = super::shared::build_owned(&base, semantic, budget)?;
    super::shared::push_dedup(&mut owned, checked.record().record_digest.as_str(), budget)?;
    Ok(owned)
}
