use super::super::DocumentaryParticipationRow;
use super::{invalid, object};
use crate::export::ExportDataset;
use crate::grouping::Admission;
use crate::report::{documentary_source_error as source, ReportResult};
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, discover_frozen_html_results,
    with_current_attached_participation, with_frozen_attached_participation,
    AttachedParticipationV1, AttachmentHtmlResultDiscoveryV1,
};
use census_domain::affiliation_candidates::DeclaredFrozenView;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_domain::model::{PerformanceId, QueryProfileV1};
use census_domain::participation_candidates::{CandidateBinding, CandidateQuery};
use census_store::{FrozenEvidenceSubjectAttachmentBundleV1, StoreSnapshot};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::path::Path;

mod assessment;
mod diagnostics;
mod facts;
pub(crate) use diagnostics::UnsupportedDocumentarySource;

#[derive(Clone, Copy)]
pub(super) enum AssessmentSources<'a, 'b, 's> {
    HistoricalWithheld,
    Current(&'a AttachedParticipationV1<'b, 's>),
}

#[derive(Clone, Copy)]
enum ReplaySemantics {
    HistoricalWithheld,
    Current,
}

pub(super) struct Collected {
    pub(super) rows: BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    pub(super) unsupported: Vec<UnsupportedDocumentarySource>,
}

pub(super) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    directory: &Path,
) -> ReportResult<(
    Collected,
    super::archive::attachment::AttachmentClosureDescriptor,
)> {
    let mut grant = Grant::new();
    let result = (|| {
        let accepted = dataset.verified_public_identities()?.accepted_snapshot();
        let current = snapshot
            .read_current_evidence_subject_attachment_references_with_accepted_v1(
                accepted, &mut grant,
            )?;
        let bundle = current.freeze_closure(&mut grant)?;
        let plan = discover_current_html_results(&current, &mut grant).map_err(source)?;
        let rows = with_current_attached_participation(&current, &mut grant, |sources, grant| {
            Ok(collect(
                dataset,
                profile,
                captured_at,
                &plan,
                AssessmentSources::Current(sources),
                grant,
            ))
        })
        .map_err(source)??;
        let descriptor = super::archive::capture_attachment(&bundle, directory, &mut grant)?;
        Ok((rows, descriptor))
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn replay(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    descriptor: &super::archive::attachment::AttachmentClosureDescriptor,
    directory: &Path,
) -> ReportResult<Collected> {
    replay_version(
        dataset,
        profile,
        captured_at,
        descriptor,
        directory,
        ReplaySemantics::HistoricalWithheld,
    )
}

pub(super) fn replay_current(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    descriptor: &super::archive::attachment::AttachmentClosureDescriptor,
    directory: &Path,
) -> ReportResult<Collected> {
    replay_version(
        dataset,
        profile,
        captured_at,
        descriptor,
        directory,
        ReplaySemantics::Current,
    )
}

fn replay_version(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    descriptor: &super::archive::attachment::AttachmentClosureDescriptor,
    directory: &Path,
    semantics: ReplaySemantics,
) -> ReportResult<Collected> {
    let mut grant = Grant::new();
    let result = (|| {
        let bundle: FrozenEvidenceSubjectAttachmentBundleV1 =
            super::archive::replay_attachment(descriptor, directory, &mut grant)?;
        let accepted = dataset.verified_public_identities()?.accepted_snapshot();
        let current = census_store::replay_frozen_evidence_subject_attachment_references_v1(
            &bundle, accepted, &mut grant,
        )?;
        let plan = discover_frozen_html_results(&current, &mut grant).map_err(source)?;
        match semantics {
            ReplaySemantics::HistoricalWithheld => collect(
                dataset,
                profile,
                captured_at,
                &plan,
                AssessmentSources::HistoricalWithheld,
                &mut grant,
            ),
            ReplaySemantics::Current => {
                with_frozen_attached_participation(&current, &mut grant, |sources, grant| {
                    Ok(collect(
                        dataset,
                        profile,
                        captured_at,
                        &plan,
                        AssessmentSources::Current(sources),
                        grant,
                    ))
                })
                .map_err(source)?
            }
        }
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

fn collect(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    plan: &AttachmentHtmlResultDiscoveryV1<'_, '_, '_>,
    sources: AssessmentSources<'_, '_, '_>,
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
    let unsupported = diagnostics::collect(plan, grant, &mut budget)?;
    let mut rows = BTreeMap::new();
    for index in 0..plan.group_count() {
        let result = plan
            .with_group(index, grant, |linked, grant| {
                Ok((|| -> ReportResult<()> {
                    for row_index in 0..linked.row_count() {
                        let row = assessment::row(
                            &linked,
                            row_index,
                            query,
                            sources,
                            grant,
                            &mut budget,
                        )?;
                        reject_collision(dataset, &rows, row.performance_id(), &mut budget)?;
                        let search = rows
                            .len()
                            .checked_add(1)
                            .ok_or(census_store::StoreError::CounterOverflow)?;
                        let owner =
                            std::mem::size_of::<(PerformanceId, DocumentaryParticipationRow)>()
                                .checked_add(row.performance_id().as_str().len())
                                .ok_or(census_store::StoreError::CounterOverflow)?;
                        budget.charge(owner, search)?;
                        rows.insert(row.performance_id().clone(), row);
                    }
                    Ok(())
                })())
            })
            .map_err(source)?;
        result?;
    }
    Ok(Collected { rows, unsupported })
}

fn reject_collision(
    dataset: &ExportDataset,
    rows: &BTreeMap<PerformanceId, DocumentaryParticipationRow>,
    id: &PerformanceId,
    budget: &mut Admission,
) -> ReportResult<()> {
    for performance in &dataset.performances {
        budget.charge(0, 1)?;
        if &performance.id == id {
            return Err(invalid("documentary performance collides with structured source").into());
        }
    }
    for prior in rows.keys() {
        budget.charge(0, 1)?;
        if prior == id {
            return Err(invalid(
                "documentary performance has duplicate or conflicting source observations",
            )
            .into());
        }
    }
    Ok(())
}

fn domain(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> census_store::StoreError {
    census_store::EvidenceSubjectAttachmentStoreError::from(error).into()
}
fn step(grant: &mut Grant, count: usize) -> ReportResult<()> {
    grant.charge_steps(count).map_err(domain)?;
    Ok(())
}

#[cfg(test)]
mod tests;
