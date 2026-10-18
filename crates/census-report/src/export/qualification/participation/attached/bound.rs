use super::super::{invalid, object};
use super::Collected;
use crate::export::ExportDataset;
use crate::grouping::Admission;
use crate::report::{documentary_source_error as source, ReportResult};
use census_crawl::participation_evidence::recorded::{
    discover_frozen_html_results, with_frozen_attached_participation,
};
use census_domain::affiliation_candidates::DeclaredFrozenView;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_domain::model::QueryProfileV1;
use census_domain::participation_candidates::{CandidateBinding, CandidateQuery};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::num::NonZeroU64;

pub(super) fn collect(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    school: &census_store::ReplayedResultSchoolCorrespondencesV1<'_>,
    grant: &mut Grant,
) -> ReportResult<Collected> {
    let profile_id = profile.id().map_err(census_store::StoreError::from)?;
    let binding = CandidateBinding {
        profile: &profile_id,
        participation_rule: &profile.input().policies.participation,
        frozen: DeclaredFrozenView {
            lineage: object(&dataset.lineage.store_identity)?,
            revision: NonZeroU64::new(dataset.lineage.snapshot_sequence)
                .ok_or_else(|| invalid("empty documentary snapshot revision"))?,
            captured_at,
            closure_object: object(&dataset.lineage.input_digest)?,
            family_inventory: object(&dataset.lineage.source_digest)?,
        },
    };
    let query = CandidateQuery { profile, binding };
    let mut budget = Admission::new("selected documentary observations");
    let attachments = school.attachments(grant)?;
    let plan = discover_frozen_html_results(attachments, grant).map_err(source)?;
    let unsupported = super::diagnostics::collect(&plan, grant, &mut budget)?;
    let mut rows = BTreeMap::new();
    let mut mapped = BTreeMap::new();
    with_frozen_attached_participation(attachments, grant, |sources, grant| {
        Ok(collect_groups(
            dataset,
            query,
            school,
            sources,
            &plan,
            grant,
            &mut budget,
            &mut rows,
            &mut mapped,
        ))
    })
    .map_err(source)??;
    Ok(Collected {
        rows,
        unsupported,
        mapped,
        documentary_authority: Some(
            super::super::current_authority::DocumentaryAuthority::from_school(school, grant)?,
        ),
    })
}

fn collect_groups(
    dataset: &ExportDataset,
    query: CandidateQuery<'_>,
    school: &census_store::ReplayedResultSchoolCorrespondencesV1<'_>,
    sources: &census_crawl::participation_evidence::recorded::AttachedParticipationV1<'_, '_>,
    plan: &census_crawl::participation_evidence::recorded::AttachmentHtmlResultDiscoveryV1<
        '_,
        '_,
        '_,
    >,
    grant: &mut Grant,
    budget: &mut Admission,
    rows: &mut BTreeMap<
        census_domain::model::PerformanceId,
        crate::export::qualification::DocumentaryParticipationRow,
    >,
    mapped: &mut BTreeMap<
        census_domain::model::PerformanceId,
        super::super::PinnedParticipationRow,
    >,
) -> ReportResult<()> {
    for index in 0..plan.group_count() {
        let result = plan
            .with_group(index, grant, |linked, grant| {
                Ok(collect_group(
                    dataset, query, school, sources, &linked, grant, budget, rows, mapped,
                ))
            })
            .map_err(source)?;
        result?;
    }
    Ok(())
}

fn collect_group(
    dataset: &ExportDataset,
    query: CandidateQuery<'_>,
    school: &census_store::ReplayedResultSchoolCorrespondencesV1<'_>,
    sources: &census_crawl::participation_evidence::recorded::AttachedParticipationV1<'_, '_>,
    linked: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1<
        '_,
        '_,
    >,
    grant: &mut Grant,
    budget: &mut Admission,
    rows: &mut BTreeMap<
        census_domain::model::PerformanceId,
        crate::export::qualification::DocumentaryParticipationRow,
    >,
    mapped: &mut BTreeMap<
        census_domain::model::PerformanceId,
        super::super::PinnedParticipationRow,
    >,
) -> ReportResult<()> {
    for row_index in 0..linked.row_count() {
        let checked = census_crawl::participation_evidence::result_ownership::school_correspondence::read::checked_frozen(
            school, linked, row_index, grant,
        )
        .map_err(source)?;
        match checked {
            Some(checked) => super::mapped::insert(
                dataset, query, sources, linked, row_index, &checked, grant, budget, rows, mapped,
            )?,
            None => {
                let row = super::assessment::row(
                    linked,
                    row_index,
                    query,
                    super::AssessmentSources::Current(sources),
                    grant,
                    budget,
                )?;
                super::reject_extended(dataset, rows, mapped, row.performance_id(), budget)?;
                let search = rows
                    .len()
                    .checked_add(mapped.len())
                    .and_then(|count| count.checked_add(1))
                    .ok_or(census_store::StoreError::CounterOverflow)?;
                let owner = std::mem::size_of::<(
                    census_domain::model::PerformanceId,
                    crate::export::qualification::DocumentaryParticipationRow,
                )>()
                .checked_add(row.performance_id().as_str().len())
                .ok_or(census_store::StoreError::CounterOverflow)?;
                budget.charge(owner, search)?;
                rows.insert(row.performance_id().clone(), row);
            }
        }
    }
    Ok(())
}
