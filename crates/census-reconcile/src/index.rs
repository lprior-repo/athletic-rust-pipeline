use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, NaturalKey, RetainedConflict, ReviewCase,
    ReviewState, SourceEntityKind, SourceIdentity, SourceObjectIdentity,
};
use std::collections::HashMap;

use census_report::report::ReportResult;
use census_store::{Entity, Store, Table};

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

fn stage_digest(store: &Store) -> ReportResult<String> {
    let inputs: Vec<Table> = Table::ALL
        .into_iter()
        .filter(|table| !STAGE_OUTPUTS.contains(table))
        .collect();
    Ok(store.snapshot().tables_digest(&inputs)?)
}

fn recorded_report(_store: &Store) -> ReportResult<IndexReport> {
    Ok(IndexReport {
        snapshots: 1,
        ..IndexReport::default()
    })
}

fn stored_cases(store: &Store) -> ReportResult<HashMap<String, ReviewCase>> {
    Ok(store
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

fn supersede(
    store: &Store,
    stored: &HashMap<String, ReviewCase>,
    current: &[ReviewCase],
) -> census_store::StoreResult<usize> {
    let live: std::collections::HashSet<&str> =
        current.iter().map(|case| case.id.as_str()).collect();
    let stale: Vec<ReviewCase> = stored
        .values()
        .filter(|case| case.state == ReviewState::Pending && !live.contains(case.id.as_str()))
        .cloned()
        .map(|mut case| {
            case.state = ReviewState::Superseded;
            case
        })
        .collect();
    let closed = stale.len();
    store.replace_many(Table::ReviewCases, &stale)?;
    Ok(closed)
}

pub fn derive(store: &Store, phase: &str, finished_at: &str) -> ReportResult<IndexReport> {
    let digest = stage_digest(store)?;
    let operation = format!("{STAGE_RECEIPT}:{digest}");
    if store
        .receipt(&operation)?
        .is_some_and(|held| held.digest == digest)
    {
        store.replace(Table::Snapshots, &snapshot_row(store, phase, finished_at)?)?;
        return recorded_report(store);
    }
    let pass = canonical_pass(store)?;
    let identity_applications = apply::apply_decisions(store, finished_at)?;
    let dataset = census_report::export::ExportDataset::load(store)?;
    let retained = census_report::workbook::retained_records(&dataset)?;
    let coverage = coverage_rows(&dataset, &pass.identities)?;

    let mut conflicts: Vec<RetainedConflict> = retained
        .conflicts
        .iter()
        .map(|(family, row)| {
            RetainedConflict::new(family, &row.subject_id, &row.subject, &row.detail)
        })
        .collect();
    conflicts.extend(pass.collisions);
    let stored = stored_cases(store)?;
    let reviews: Vec<ReviewCase> = retained
        .reviews
        .iter()
        .map(|(family, row)| {
            let minted = ReviewCase::minted(family, &row.subject_id, &row.subject, &row.detail);
            carried(&stored, minted)
        })
        .collect();

    store.replace_many(Table::SourceIdentities, &pass.identities)?;
    store.replace_many(Table::Conflicts, &conflicts)?;
    store.replace_many(Table::ReviewCases, &reviews)?;
    store.replace_many(Table::Coverage, &coverage)?;
    let superseded = supersede(store, &stored, &reviews)?;

    store.replace(Table::Snapshots, &snapshot_row(store, phase, finished_at)?)?;
    store.write_batch().commit_once(&operation, &digest)?;

    Ok(IndexReport {
        source_identities: pass.identities.len(),
        conflicts: conflicts.len(),
        reviews: reviews.len(),
        coverage: coverage.len(),
        snapshots: 1,
        superseded,
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

fn canonical_pass(store: &Store) -> ReportResult<CanonicalPass> {
    let mut pass = CanonicalPass::default();
    absorb(&mut pass, store.scan::<CanonicalSchool>(Table::Schools)?);
    absorb(&mut pass, store.scan::<CanonicalTeam>(Table::Teams)?);
    absorb(&mut pass, store.scan::<CanonicalCoach>(Table::Coaches)?);
    absorb(&mut pass, store.scan::<CanonicalAthlete>(Table::Athletes)?);
    absorb(&mut pass, store.scan::<CanonicalMeet>(Table::Meets)?);
    take_collisions(
        &mut pass,
        store.scan::<CanonicalEvent>(Table::Events)?.iter(),
    );
    take_collisions(
        &mut pass,
        store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .iter(),
    );
    Ok(pass)
}

fn absorb<T: IdentityBearing + NaturalKey>(pass: &mut CanonicalPass, entities: Vec<T>) {
    for entity in &entities {
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
    }
    take_collisions(pass, entities.iter());
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

fn snapshot_row(store: &Store, phase: &str, finished_at: &str) -> ReportResult<CollectionSnapshot> {
    let stats = store.stats()?;
    let mut snapshot = CollectionSnapshot::new(phase, finished_at);
    for (table, observations) in stats.appended {
        snapshot.observations.insert(table, observations);
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests;
