//! The durable indexes derived from the store's merged rows (§29-§31).
//!
//! Fjall is the system of record, so the census keeps its derived state in the same store as its
//! evidence: the source-object identities that join a provider's own ids to canonical rows, the
//! conflict and review queues it retained, per-jurisdiction and per-source coverage, and one snapshot
//! per derivation pass. Every one of those ids is a function of the finding it names, and the rows are
//! replaced rather than appended, so a pass may be repeated without minting a second identity and
//! without growing the table, and a later read is a scan of one table rather than a re-derivation of
//! the whole store.
//!
//! The derivation reuses the code that renders these values: the queues come from the same families
//! the workbook sheets print, and the jurisdiction rows come from the same reconciled coverage report
//! the census publishes. A reader of the store and a reader of the workbook therefore cannot be shown
//! different findings — they are the same computation, written twice.
//!
//! A pass never fails the pipeline for a row it cannot derive: an identity that no source supplied is
//! absent rather than invented, and a table the store holds nothing in contributes no rows.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow, CoverageScope, GradYear,
    NaturalKey, RetainedConflict, ReviewCase, SourceEntityKind, SourceIdentity, SourceNamespace,
    SourceObjectIdentity,
};
use std::collections::BTreeMap;

use crate::report::{coverage_report, ReportResult};
use census_store::{Store, Table};

/// What one derivation pass wrote, per table.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IndexReport {
    pub source_identities: usize,
    pub conflicts: usize,
    pub reviews: usize,
    pub coverage: usize,
    pub snapshots: usize,
}

impl IndexReport {
    /// Every row the pass wrote.
    pub const fn total(&self) -> usize {
        self.source_identities
            .saturating_add(self.conflicts)
            .saturating_add(self.reviews)
            .saturating_add(self.coverage)
            .saturating_add(self.snapshots)
    }
}

/// Derive every index from the merged rows and append it.
///
/// `phase` names the pass that produced the rows (`run`, `index`, a source id) and `finished_at` is
/// the date it finished; together they key the snapshot, so a store carries one snapshot per phase
/// per day holding the latest counts for it.
pub fn derive(store: &Store, phase: &str, finished_at: &str) -> ReportResult<IndexReport> {
    let pass = canonical_pass(store)?;
    let retained = crate::workbook::retained_records(store)?;
    let coverage = coverage_rows(store, &pass.identities)?;

    let mut conflicts: Vec<RetainedConflict> = retained
        .conflicts
        .iter()
        .map(|(family, row)| {
            RetainedConflict::new(family, &row.subject_id, &row.subject, &row.detail)
        })
        .collect();
    // The collisions the canonical rows' own merges retained join the same queue: a canonical id two
    // natural keys minted is a retained conflict like any other, and it has to reach a reader of the
    // store, not only a reader of the row that kept it.
    conflicts.extend(pass.collisions);
    let reviews: Vec<ReviewCase> = retained
        .reviews
        .iter()
        .map(|(family, row)| {
            ReviewCase::pending(family, &row.subject_id, &row.subject, &row.detail)
        })
        .collect();

    // Derived tables are replaced rather than appended: their ids are functions of the findings they
    // name, so a repeated pass overwrites the row it wrote before instead of adding another copy.
    store.replace_many(Table::SourceIdentities, &pass.identities)?;
    store.replace_many(Table::Conflicts, &conflicts)?;
    store.replace_many(Table::ReviewCases, &reviews)?;
    store.replace_many(Table::Coverage, &coverage)?;

    // The snapshot is taken after every append above: the counters it records are the store's
    // cumulative appended-observation totals once this pass finished, which is what a later reader
    // asking "how much is in the store, and as of when" needs.
    store.replace(Table::Snapshots, &snapshot_row(store, phase, finished_at)?)?;

    Ok(IndexReport {
        source_identities: pass.identities.len(),
        conflicts: conflicts.len(),
        reviews: reviews.len(),
        coverage: coverage.len(),
        snapshots: 1,
    })
}

/// A canonical row that carries the provider identities observed for it.
trait IdentityBearing {
    const KIND: SourceEntityKind;
    fn canonical_id(&self) -> &str;
    fn source_identities(&self) -> &[SourceIdentity];
}

macro_rules! identity_bearing {
    ($type:ty, $kind:expr) => {
        impl IdentityBearing for $type {
            const KIND: SourceEntityKind = $kind;

            fn canonical_id(&self) -> &str {
                self.id.as_str()
            }

            fn source_identities(&self) -> &[SourceIdentity] {
                &self.source_identities
            }
        }
    };
}

identity_bearing!(CanonicalSchool, SourceEntityKind::Schools);
identity_bearing!(CanonicalTeam, SourceEntityKind::Teams);
identity_bearing!(CanonicalCoach, SourceEntityKind::Coaches);
identity_bearing!(CanonicalAthlete, SourceEntityKind::Athletes);
identity_bearing!(CanonicalMeet, SourceEntityKind::Meets);

/// What one pass over the canonical tables yields: the provider identities the rows name, and the
/// canonical-id collisions their own merges retained.
///
/// Both answers come from the same scan of each table, because a collision is only ever visible on
/// the row that survived it: the merge that raised the finding is the merge that produced the row, so
/// asking twice would decode the whole store a second time to answer a question only these rows know.
#[derive(Default)]
struct CanonicalPass {
    identities: Vec<SourceObjectIdentity>,
    collisions: Vec<RetainedConflict>,
}

/// Read every canonical table once: §31's join rows and the collisions the rows retained.
///
/// Rows are written in table order and then by canonical id, so two stores holding the same entities
/// derive the same batch whatever order their observation keys were appended in.
fn canonical_pass(store: &Store) -> ReportResult<CanonicalPass> {
    let mut pass = CanonicalPass::default();
    absorb(&mut pass, store.scan::<CanonicalSchool>(Table::Schools)?);
    absorb(&mut pass, store.scan::<CanonicalTeam>(Table::Teams)?);
    absorb(&mut pass, store.scan::<CanonicalCoach>(Table::Coaches)?);
    absorb(&mut pass, store.scan::<CanonicalAthlete>(Table::Athletes)?);
    absorb(&mut pass, store.scan::<CanonicalMeet>(Table::Meets)?);
    // Events and performances name no provider identity of their own — §31 carries an event's and a
    // performance's provider ids — so they contribute only the collisions their merges retained.
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

/// Append one table's provider identities to a pass, and take the collisions its rows retained.
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

/// Take every collision one table's merged rows retained, in row order.
fn take_collisions<'a, T: NaturalKey + 'a>(
    pass: &mut CanonicalPass,
    entities: impl IntoIterator<Item = &'a T>,
) {
    for entity in entities {
        pass.collisions
            .extend(entity.retained_conflicts().iter().cloned());
    }
}

/// Coverage rows: one per jurisdiction from the reconciled coverage report, one per source namespace
/// from the identities this pass read.
fn coverage_rows(
    store: &Store,
    identities: &[SourceObjectIdentity],
) -> ReportResult<Vec<CoverageRow>> {
    let report = coverage_report(store, Some(GradYear::CO2027.get()))?;
    let mut rows = Vec::with_capacity(report.jurisdictions.len().saturating_add(identities.len()));
    for jurisdiction in &report.jurisdictions {
        rows.push(
            CoverageRow::new(
                CoverageScope::Jurisdiction,
                jurisdiction.jurisdiction.to_string(),
            )
            .with("schools", count(jurisdiction.schools))
            .with(
                "schools_with_athletes",
                count(jurisdiction.schools_with_athletes),
            )
            .with("cohort_athletes", count(jurisdiction.athletes))
            .with("boys", count(jurisdiction.boys))
            .with("girls", count(jurisdiction.girls))
            .with("grad_verified", count(jurisdiction.grad_verified))
            .with("grad_unresolved", count(jurisdiction.grad_unresolved))
            .with("multisource", count(jurisdiction.multisource))
            .with("with_performance", count(jurisdiction.with_performance))
            .with("with_profile_url", count(jurisdiction.with_profile_url))
            .with(
                "schools_with_tf_coach",
                count(jurisdiction.schools_with_tf_coach),
            )
            .with(
                "schools_with_xc_coach",
                count(jurisdiction.schools_with_xc_coach),
            )
            .with(
                "schools_with_coach_email",
                count(jurisdiction.schools_with_coach_email),
            )
            .with("meets", count(jurisdiction.meets))
            .with("coaches", count(jurisdiction.coaches))
            .with("coaches_with_email", count(jurisdiction.coaches_with_email)),
        );
    }
    rows.extend(source_coverage(identities));
    Ok(rows)
}

/// One coverage row per source namespace: how many source objects it contributes, per canonical
/// table.
fn source_coverage(identities: &[SourceObjectIdentity]) -> Vec<CoverageRow> {
    let mut by_source: BTreeMap<&SourceNamespace, BTreeMap<String, u64>> = BTreeMap::new();
    for identity in identities {
        let metrics = by_source.entry(&identity.namespace).or_default();
        bump(metrics, "identities");
        bump(metrics, identity.entity.slug());
    }
    by_source
        .into_iter()
        .map(|(namespace, metrics)| {
            let source = namespace.to_string();
            CoverageRow {
                id: format!("source:{source}"),
                scope: CoverageScope::Source,
                subject: source,
                metrics,
            }
        })
        .collect()
}

/// Count one occurrence, saturating rather than wrapping.
fn bump(metrics: &mut BTreeMap<String, u64>, metric: &str) {
    let slot = metrics.entry(metric.to_string()).or_default();
    *slot = slot.saturating_add(1);
}

/// The snapshot of this pass: every table's exact appended-observation counter, read once the pass
/// has appended its rows, so the row states the store's cumulative total at the end of the pass.
fn snapshot_row(store: &Store, phase: &str, finished_at: &str) -> ReportResult<CollectionSnapshot> {
    let stats = store.stats()?;
    let mut snapshot = CollectionSnapshot::new(phase, finished_at);
    for (table, observations) in stats.appended {
        snapshot.observations.insert(table, observations);
    }
    Ok(snapshot)
}

/// A count as the store publishes it. A `usize` larger than `u64` cannot exist on a 64-bit target;
/// where it could, the measurement saturates instead of wrapping into a smaller number.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
