use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, CanonicalTeam,
    CollectionSnapshot, NaturalKey, RetainedConflict, ReviewCase, ReviewState, SourceEntityKind,
    SourceIdentity, SourceObjectIdentity,
};
use std::collections::HashMap;

use census_report::report::ReportResult;
use census_store::{Entity, Store, StoreSnapshot, Table};

mod apply;
mod coverage;

#[cfg(test)]
mod application_tests;

#[cfg(test)]
mod stage_gate_tests;

use coverage::coverage_rows;

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

fn superseded(stored: &HashMap<String, ReviewCase>, current: &[ReviewCase]) -> Vec<ReviewCase> {
    let live: std::collections::HashSet<&str> =
        current.iter().map(|case| case.id.as_str()).collect();
    stored
        .values()
        .filter(|case| case.state == ReviewState::Pending && !live.contains(case.id.as_str()))
        .cloned()
        .map(|mut case| {
            case.state = ReviewState::Superseded;
            case
        })
        .collect()
}

pub fn derive(
    store: &Store,
    phase: &str,
    finished_at: &str,
    school_year: census_domain::model::SchoolYear,
) -> ReportResult<IndexReport> {
    let identity_applications = apply::apply_decisions(store, finished_at)?;
    let mut stage = store.stage_derived()?;
    let digest = stage_digest(stage.view())?;
    let operation = format!("{STAGE_RECEIPT}:{digest}");
    let dataset = census_report::export::ExportDataset::from_snapshot(store, stage.view())?;
    let pass = canonical_pass(&dataset);
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
    let closed = superseded(&stored, &reviews);
    let snapshot = snapshot_row(stage.view(), phase, finished_at)?;

    for table in STAGE_OUTPUTS {
        if table.generation_partitioned() {
            stage.carry_forward(table)?;
        }
    }
    stage.replace_many(Table::SourceIdentities, &pass.identities)?;
    stage.replace_many(Table::Conflicts, &conflicts)?;
    stage.replace_many(Table::ReviewCases, &closed)?;
    stage.replace_many(Table::ReviewCases, &reviews)?;
    stage.replace_many(Table::Coverage, &coverage)?;
    stage.replace_many(Table::Snapshots, &[snapshot])?;
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

trait IdentityBearing {
    const KIND: SourceEntityKind;
    fn canonical_id(&self) -> &str;
    fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity>;
}

macro_rules! identity_bearing {
    ($type:ty, $kind:expr) => {
        impl IdentityBearing for $type {
            const KIND: SourceEntityKind = $kind;

            fn canonical_id(&self) -> &str {
                self.id.as_str()
            }

            fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity> {
                self.source_identities.iter()
            }
        }
    };
}

identity_bearing!(CanonicalSchool, SourceEntityKind::Schools);
identity_bearing!(CanonicalTeam, SourceEntityKind::Teams);
identity_bearing!(CanonicalCoach, SourceEntityKind::Coaches);
identity_bearing!(CanonicalMeet, SourceEntityKind::Meets);

impl IdentityBearing for CanonicalAthlete {
    const KIND: SourceEntityKind = SourceEntityKind::Athletes;

    fn canonical_id(&self) -> &str {
        self.id.as_str()
    }

    fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity> {
        self.identities()
    }
}

#[derive(Default)]
struct CanonicalPass {
    identities: Vec<SourceObjectIdentity>,
    collisions: Vec<RetainedConflict>,
}

fn canonical_pass(dataset: &census_report::export::ExportDataset) -> CanonicalPass {
    let mut pass = CanonicalPass::default();
    absorb(&mut pass, dataset.schools.values());
    absorb(&mut pass, dataset.teams.values());
    absorb(&mut pass, &dataset.coaches);
    absorb(&mut pass, &dataset.athletes);
    absorb(&mut pass, &dataset.meets);
    take_collisions(&mut pass, &dataset.events);
    take_collisions(&mut pass, &dataset.performances);
    pass
}

fn absorb<'a, T: IdentityBearing + NaturalKey + 'a>(
    pass: &mut CanonicalPass,
    entities: impl IntoIterator<Item = &'a T>,
) {
    for entity in entities {
        for identity in entity.source_identities() {
            let mut row = SourceObjectIdentity::new(
                identity.namespace.clone(),
                T::KIND,
                identity.id.clone(),
                entity.canonical_id(),
            );
            if let Some(url) = identity.url.clone() {
                row = row.with_url(url);
            }
            pass.identities.push(row);
        }
        pass.collisions
            .extend(entity.retained_conflicts().iter().cloned());
    }
}

fn take_collisions<'a, T: NaturalKey + 'a>(
    pass: &mut CanonicalPass,
    entities: impl IntoIterator<Item = &'a T>,
) {
    for entity in entities {
        pass.collisions
            .extend(entity.retained_conflicts().iter().cloned());
    }
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
