use census_domain::model::{ReviewCase, SourceAccessCondition};

use super::{count, retained_access, table_rows, JournalCounts, SealRequest};
use crate::census::{
    owed_cohort_decisions, owed_identity_candidates, GapTally, OpenWork, RetainedFindings,
    SealCounts, SealEvidence, WorkbookCheck,
};
use census_report::report::{Census, CoverageReport};
use census_store::{StoreStats, Table};

pub(super) fn assemble(
    coverage: &CoverageReport,
    census: &Census,
    stats: &StoreStats,
    cases: &[ReviewCase],
    access: &[SourceAccessCondition],
    request: &SealRequest,
    workbook: WorkbookCheck,
) -> SealEvidence {
    let journal = request.journal.clone().unwrap_or_default();
    SealEvidence {
        open: open_work(cases, journal),
        counts: seal_counts(census, coverage),
        retained: retained_findings(coverage, stats, access, request),
        workbook,
        observed_on: census.generated_on.clone(),
    }
}

fn open_work(cases: &[ReviewCase], journal: JournalCounts) -> OpenWork {
    OpenWork {
        jurisdiction_sweeps: journal.jurisdiction_sweeps,
        source_objects: journal.source_objects,
        cohort_decisions: Some(owed_cohort_decisions(cases)),
        identity_candidates: Some(owed_identity_candidates(cases)),
    }
}

fn seal_counts(census: &Census, coverage: &CoverageReport) -> SealCounts {
    SealCounts {
        jurisdiction_buckets: count(census.by_state.len()),
        schools: count(coverage.read.schools),
        meets: count(coverage.read.meets),
        athletes: count(census.totals.athletes),
        class_of_2027: count(census.totals.class_of_2027),
        cohort_performances: count(coverage.read.performances),
        coaches: count(census.totals.coaches),
    }
}

fn retained_findings(
    coverage: &CoverageReport,
    stats: &StoreStats,
    access: &[SourceAccessCondition],
    request: &SealRequest,
) -> RetainedFindings {
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
        access_conditions,
        blocked_hosts,
        throttled_hosts,
        silent_sources: request
            .journal
            .as_ref()
            .map_or_else(Vec::new, |journal| journal.silent_sources.clone()),
        source_failures: request.source_failures,
        observations: stats.observations,
        calculations: count(coverage.read.performances),
    }
}
