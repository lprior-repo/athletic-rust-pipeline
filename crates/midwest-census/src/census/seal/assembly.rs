//! The evidence §70's seal certifies, assembled from the store, the classifier and the run's journal.
//!
//! Split out of `seal.rs` to keep both files inside the one-page budget: this module says *where*
//! every count is read from — the store's own ledger, the coverage classifier's report, the
//! workbook's bytes, or the run's journal — while `seal.rs` keeps the ladder and the refusal.
//! Nothing here decides whether the evidence holds: that is the seal's own comparison, and every
//! count here is read rather than claimed.

use census_domain::model::{ReviewCase, SourceAccessCondition};

use super::{count, retained_access, table_rows, JournalCounts, SealRequest};
use crate::census::{
    owed_cohort_decisions, owed_identity_candidates, GapTally, OpenWork, RetainedFindings,
    SealCounts, SealEvidence, WorkbookCheck,
};
use census_report::report::{Census, CoverageReport};
use census_store::{StoreStats, Table};

/// Everything §70 asks the census to prove, from the store, the classifier and the run's journal.
pub(super) fn assemble(
    coverage: &CoverageReport,
    census: &Census,
    stats: &StoreStats,
    cases: &[ReviewCase],
    access: &[SourceAccessCondition],
    request: &SealRequest,
    workbook: WorkbookCheck,
) -> SealEvidence {
    let journal = request.journal.unwrap_or_default();
    SealEvidence {
        open: open_work(cases, journal),
        counts: seal_counts(census, coverage),
        retained: retained_findings(coverage, stats, access, request),
        workbook,
        observed_on: census.generated_on.clone(),
    }
}

/// The work §70 leaves open: two counts only the run's journal can answer, two the store's own rows.
///
/// The two halves are not interchangeable. A jurisdiction's stages and a source object's accepted
/// observations live in the workflow journal, which only the service can read; the cohort decisions
/// and identity candidates are retained review cases, which the store is the authority on. A count
/// nobody took is `None`, and `None` keeps its item open: that is what makes an offline seal refuse
/// rather than certify a completion it never checked.
fn open_work(cases: &[ReviewCase], journal: JournalCounts) -> OpenWork {
    OpenWork {
        jurisdiction_sweeps: journal.jurisdiction_sweeps,
        source_objects: journal.source_objects,
        cohort_decisions: Some(owed_cohort_decisions(cases)),
        identity_candidates: Some(owed_identity_candidates(cases)),
    }
}

/// The store-side counts §70 certifies, each read from the census and the coverage report it came
/// from rather than from a caller's tally.
fn seal_counts(census: &Census, coverage: &CoverageReport) -> SealCounts {
    SealCounts {
        jurisdictions: count(census.by_state.len()),
        schools: count(coverage.read.schools),
        meets: count(coverage.read.meets),
        athletes: count(census.totals.athletes),
        class_of_2027: count(census.totals.class_of_2027),
        performances: count(coverage.read.performances),
        coaches: count(census.totals.coaches),
    }
}

/// The retained findings §70 publishes: the classifier's gaps, the store's own row counts, and the
/// counts only the caller's journal can answer.
fn retained_findings(
    coverage: &CoverageReport,
    stats: &StoreStats,
    access: &[SourceAccessCondition],
    request: &SealRequest,
) -> RetainedFindings {
    // The access conditions are read as rows, not as a ledger count: the split a reader needs is a
    // property of the rows' own `kind`, and one read answers both.
    let (access_conditions, blocked_hosts, throttled_hosts) = retained_access(access);
    RetainedFindings {
        gaps: coverage
            .gaps
            .iter()
            .map(|gap| GapTally {
                class: gap.class.to_string(),
                unit: gap.unit.to_string(),
                count: count(gap.count),
            })
            .collect(),
        conflicts: table_rows(stats, Table::Conflicts),
        // §70 item 2 is about operations that stopped, and a *refused host* is the store's record
        // of that; a throttle is a slowdown with a cooldown, kept apart because reading it as a
        // stopped operation would overstate what the census lost.
        access_conditions,
        blocked_hosts,
        throttled_hosts,
        // A source *failure* is a terminal outcome of one attempt, and the store keeps access
        // conditions rather than attempts. The journal knows the objects that accepted nothing,
        // which is the other half of §70 item 2's fact; per-attempt failures are reported only where
        // the caller could read them.
        source_failures: request.source_failures,
        observations: stats.observations,
        calculations: count(coverage.read.performances),
    }
}
