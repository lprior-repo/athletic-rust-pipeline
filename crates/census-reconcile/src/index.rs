use census_domain::model::{CollectionSnapshot, RetainedConflict, ReviewCase};
use std::collections::HashMap;

use census_report::report::ReportResult;
use census_store::{is_qualification_lifecycle, Entity, Store, StoreSnapshot, Table};

mod apply;
mod coverage;
mod identity_pass;
mod supersession;

#[cfg(test)]
mod application_tests;

#[cfg(test)]
mod stage_gate_tests;

use coverage::coverage_rows;
use identity_pass::canonical_pass;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IndexReport {
    pub source_identities: usize,
    pub conflicts: usize,
    pub reviews: usize,
    pub coverage: usize,
    pub snapshots: usize,
    pub superseded: usize,
    pub identity_applications: usize,
}

impl IndexReport {
    pub const fn total(&self) -> usize {
        self.source_identities
            .saturating_add(self.conflicts)
            .saturating_add(self.reviews)
            .saturating_add(self.coverage)
            .saturating_add(self.snapshots)
            .saturating_add(self.identity_applications)
    }
}

const STAGE_RECEIPT: &str = "index-stage:v2";

const STAGE_OUTPUTS: [Table; 6] = [
    Table::SourceIdentities,
    Table::Conflicts,
    Table::ReviewCases,
    Table::Coverage,
    Table::Snapshots,
    Table::AthleteIdentityDecisions,
];

fn stage_digest(snapshot: &StoreSnapshot<'_>) -> ReportResult<String> {
    let inputs: Vec<Table> = Table::ALL
        .into_iter()
        .filter(|table| !STAGE_OUTPUTS.contains(table))
        .collect();
    Ok(snapshot.tables_digest(&inputs)?)
}

fn stored_cases(snapshot: &StoreSnapshot<'_>) -> ReportResult<HashMap<String, ReviewCase>> {
    Ok(snapshot
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .map(|case| (case.id.clone(), case))
        .collect())
}

fn carried(stored: &HashMap<String, ReviewCase>, minted: ReviewCase) -> ReviewCase {
    match stored.get(&minted.id) {
        Some(case) => {
            let mut case = case.clone();
            case.merge(minted);
            case
        }
        None => minted,
    }
}

pub fn derive(
    store: &Store,
    phase: &str,
    finished_at: &str,
    school_year: census_domain::model::SchoolYear,
) -> ReportResult<IndexReport> {
    store.store_identity()?;
    let identity_applications =
        if is_qualification_lifecycle(store.store_format()?.lifecycle_revision) {
            0
        } else {
            apply::apply_decisions(store, finished_at)?
        };
    let mut stage = store.stage_derived()?;
    let base = stage_digest(stage.view())?;
    let dataset = census_report::export::ExportDataset::from_snapshot(store, stage.view())?;
    let pass = canonical_pass(&dataset);
    let retained = census_report::workbook::retained_records(&dataset, school_year, &[])?;
    let coverage = coverage_rows(&dataset, &pass.identities)?;

    let mut conflicts: Vec<RetainedConflict> = retained
        .conflicts
        .iter()
        .map(|(family, row)| {
            RetainedConflict::new(family, &row.subject_id, &row.subject, &row.detail)
        })
        .collect();
    conflicts.extend(pass.collisions);
    let stored = stored_cases(stage.view())?;
    let reviews: Vec<ReviewCase> = retained
        .reviews
        .iter()
        .map(|(family, row)| {
            let minted = ReviewCase::minted(family, &row.subject_id, &row.subject, &row.detail);
            carried(&stored, minted)
        })
        .collect();
    let (closed, provenance) = supersession::plan(stage.view(), &stored, &reviews, finished_at)?;
    let digest =
        supersession::publication_digest(stage.view(), &stored, &base, &provenance, school_year)?;
    let operation = format!("{STAGE_RECEIPT}:{digest}");
    let snapshot = snapshot_row(stage.view(), phase, finished_at)?;

    carry_outputs(&mut stage)?;
    stage.replace_many(Table::SourceIdentities, &pass.identities)?;
    stage.replace_many(Table::Conflicts, &conflicts)?;
    supersession::stage_findings(&mut stage, &closed, &reviews, provenance)?;
    stage.replace_many(Table::Coverage, &coverage)?;
    stage.replace_many(Table::Snapshots, &[snapshot])?;

    let publication = stage.publish(&operation, &digest)?;

    Ok(IndexReport {
        source_identities: pass.identities.len(),
        conflicts: conflicts.len(),
        reviews: reviews.len(),
        coverage: coverage.len(),
        snapshots: 1,
        superseded: if publication.application.written() {
            closed.len()
        } else {
            0
        },
        identity_applications,
    })
}

fn snapshot_row(
    view: &StoreSnapshot<'_>,
    phase: &str,
    finished_at: &str,
) -> ReportResult<CollectionSnapshot> {
    let mut snapshot = CollectionSnapshot::new(phase, finished_at);
    for (table, observations) in view.appended_counts()? {
        snapshot.observations.insert(table, observations);
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests;

fn carry_outputs(stage: &mut census_store::DerivedStage<'_>) -> ReportResult<()> {
    for table in STAGE_OUTPUTS {
        if table.generation_partitioned() {
            stage.carry_forward(table)?;
        }
    }
    Ok(())
}
