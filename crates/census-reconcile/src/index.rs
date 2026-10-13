use census_domain::model::{
    CollectionSnapshot, RetainedConflict, ReviewCase, ReviewState, SourceObjectIdentity,
    ATHLETE_IDENTITY_FAMILY,
};
use std::collections::HashMap;

use census_report::report::ReportResult;
use census_store::{Entity, Store, StoreSnapshot, Table};

mod apply;
mod coverage;
mod identity_pass;

#[cfg(test)]
mod application_tests;

#[cfg(test)]
mod stage_gate_tests;

use coverage::coverage_rows;
use identity_pass::{canonical_pass, CanonicalPass};

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

const STAGE_RECEIPT: &str = "index-stage";

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

fn superseded(
    stored: &HashMap<String, ReviewCase>,
    current: &[ReviewCase],
) -> (Vec<ReviewCase>, Vec<SupersessionRecord>) {
    let live: std::collections::HashSet<&str> =
        current.iter().map(|case| case.id.as_str()).collect();
    let mut closed = Vec::new();
    let mut provenance = Vec::new();
    for stored_case in stored.values() {
        if stored_case.state == ReviewState::Pending && !live.contains(stored_case.id.as_str()) {
            let mut case = stored_case.clone();
            case.state = ReviewState::Superseded;
            closed.push(case);
        }
    }
    for stored_case in stored.values().filter(|case| {
        case.state == ReviewState::Retained
            && case.family == ATHLETE_IDENTITY_FAMILY
            && !case.member_ids.is_empty()
            && !live.contains(case.id.as_str())
    }) {
        let same_cluster = |case: &ReviewCase| {
            case.family == ATHLETE_IDENTITY_FAMILY
                && (case.subject_id == stored_case.subject_id
                    || case
                        .member_ids
                        .iter()
                        .any(|member| stored_case.member_ids.contains(member)))
        };
        let successor = current
            .iter()
            .find(|case| case.state == ReviewState::Resolved && same_cluster(case))
            .or_else(|| {
                stored
                    .values()
                    .find(|case| case.state == ReviewState::Resolved && same_cluster(case))
            });
        let Some(successor) = successor else {
            continue;
        };
        let mut case = stored_case.clone();
        case.state = ReviewState::Superseded;
        closed.push(case);
        provenance.push(SupersessionRecord {
            old_id: stored_case.id.clone(),
            new_id: successor.id.clone(),
            reason: "obsolete machine-generated identity finding replaced by the current-policy resolution for the same cluster".to_string(),
        });
    }
    (closed, provenance)
}

#[derive(Debug)]
struct SupersessionRecord {
    old_id: String,
    new_id: String,
    reason: String,
}

fn prepare_stage<'s>(
    store: &'s Store,
    finished_at: &str,
) -> ReportResult<(
    usize,
    census_store::DerivedStage<'s>,
    String,
    String,
    census_report::export::ExportDataset,
    CanonicalPass,
)> {
    store.store_identity()?;
    let identity_applications = apply::apply_decisions(store, finished_at)?;
    let stage = store.stage_derived()?;
    let digest = stage_digest(stage.view())?;
    let operation = format!("{STAGE_RECEIPT}:{digest}");
    let dataset = census_report::export::ExportDataset::from_snapshot(store, stage.view())?;
    let pass = canonical_pass(&dataset);
    Ok((identity_applications, stage, digest, operation, dataset, pass))
}

fn write_stage_outputs(
    stage: &mut census_store::DerivedStage<'_>,
    identities: &[SourceObjectIdentity],
    conflicts: &[RetainedConflict],
    closed: &[ReviewCase],
    reviews: &[ReviewCase],
    coverage: &[census_domain::model::CoverageRow],
    snapshot: CollectionSnapshot,
) -> ReportResult<()> {
    for table in STAGE_OUTPUTS {
        if table.generation_partitioned() {
            stage.carry_forward(table)?;
        }
    }
    stage.replace_many(Table::SourceIdentities, identities)?;
    stage.replace_many(Table::Conflicts, conflicts)?;
    stage.replace_many(Table::ReviewCases, closed)?;
    stage.replace_many(Table::ReviewCases, reviews)?;
    stage.replace_many(Table::Coverage, coverage)?;
    stage.replace_many(Table::Snapshots, &[snapshot])?;
    Ok(())
}

pub fn derive(
    store: &Store,
    phase: &str,
    finished_at: &str,
    school_year: census_domain::model::SchoolYear,
) -> ReportResult<IndexReport> {
    let (identity_applications, mut stage, digest, operation, dataset, pass) =
        prepare_stage(store, finished_at)?;
    let retained = census_report::workbook::retained_records(&dataset, school_year)?;
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
    let (closed, provenance) = superseded(&stored, &reviews);
    let snapshot = snapshot_row(stage.view(), phase, finished_at)?;

    write_stage_outputs(
        &mut stage,
        &pass.identities,
        &conflicts,
        &closed,
        &reviews,
        &coverage,
        snapshot,
    )?;

    for record in provenance {
        let mut batch = store.write_batch();
        batch.journal_done(
            "identity_retained_supersession_v1",
            &record.old_id,
            &(record.new_id, record.reason),
        )?;
        batch.commit()?;
    }

    stage.publish(&operation, &digest)?;

    Ok(IndexReport {
        source_identities: pass.identities.len(),
        conflicts: conflicts.len(),
        reviews: reviews.len(),
        coverage: coverage.len(),
        snapshots: 1,
        superseded: closed.len(),
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
