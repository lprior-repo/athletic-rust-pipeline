//! Aggregation: the per-state outcomes folded into one report, and the append logs into snapshots.
//!
//! `summarize_states` owns the report totals, the per-state row order and the failure list — the
//! caller keeps its journal and re-runs, so a partial walk is reported rather than discarded — and
//! `consolidate` merges the append logs into the `out/*.jsonl` read model, counting what it wrote.

use census_crawl::CrawlResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition,
};
use census_domain::UsJurisdiction;
use census_store::{Entity, Store, StoreError, StoreResult, Table};
use std::path::Path;
use tracing::info;

use super::{CollectReport, StateProgress};

/// `usize` -> `u64` for the report counters, saturating where the value cannot fit.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Fold the per-state outcomes into one report, returning it with the states that failed.
///
/// Bounded by the number of requested jurisdictions.
pub(super) fn summarize_states(
    results: Vec<(UsJurisdiction, CrawlResult<StateProgress>)>,
) -> (CollectReport, Vec<String>) {
    let mut report = CollectReport {
        states: Vec::new(),
        teams_total: 0,
        rosters_fetched: 0,
        athletes_total: 0,
        class_of_2027_total: 0,
        requests: 0,
        cache_hits: 0,
        errors: 0,
        elapsed_seconds: 0.0,
        access_conditions: Vec::new(),
        blocked_hosts: Vec::new(),
    };
    let mut failures = Vec::new();
    for (jurisdiction, outcome) in results {
        match outcome {
            Ok(progress) => {
                report.teams_total = report.teams_total.saturating_add(progress.teams);
                report.rosters_fetched =
                    report.rosters_fetched.saturating_add(progress.rosters_done);
                report.athletes_total = report.athletes_total.saturating_add(progress.athletes);
                report.class_of_2027_total = report
                    .class_of_2027_total
                    .saturating_add(progress.class_of_2027);
                report.errors = report.errors.saturating_add(count(progress.errors.len()));
                info!(
                    state = jurisdiction.code(),
                    teams = progress.teams,
                    rosters = progress.rosters_done,
                    co2027 = progress.class_of_2027,
                    "state complete"
                );
                report.states.push(progress);
            }
            Err(error) => failures.push(format!("{}: {error}", jurisdiction.code())),
        }
    }
    report.states.sort_by_key(|progress| progress.jurisdiction);
    (report, failures)
}

/// Merge append logs into snapshots under `out/`, returning per-table counts.
pub fn consolidate(store: &Store) -> StoreResult<Vec<(String, usize)>> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out).map_err(|source| StoreError::Io {
        path: out.clone(),
        source,
    })?;
    let mut counts = bulk_counts(store, &out)?;
    counts.extend(finding_counts(store, &out)?);
    Ok(counts)
}

/// The canonical tables: one snapshot per entity table, plus the coach mailboxes the merge withheld.
fn bulk_counts(store: &Store, out: &Path) -> StoreResult<Vec<(String, usize)>> {
    let coaches_path = out.join("coaches.jsonl");
    let coaches = store.consolidate::<CanonicalCoach>(Table::Coaches, &coaches_path)?;
    Ok(vec![
        (
            "schools".to_string(),
            table_rows::<CanonicalSchool>(store, Table::Schools, &out.join("schools.jsonl"))?,
        ),
        (
            "teams".to_string(),
            table_rows::<CanonicalTeam>(store, Table::Teams, &out.join("teams.jsonl"))?,
        ),
        ("coaches".to_string(), coaches.rows),
        // The merge withholds consumer mailboxes before the snapshot is written, so this counts the
        // same rule the report and the workbook already went through.
        ("coaches_email_withheld".to_string(), coaches.withheld),
        (
            "athletes".to_string(),
            table_rows::<CanonicalAthlete>(store, Table::Athletes, &out.join("athletes.jsonl"))?,
        ),
        (
            "meets".to_string(),
            table_rows::<CanonicalMeet>(store, Table::Meets, &out.join("meets.jsonl"))?,
        ),
        (
            "events".to_string(),
            table_rows::<CanonicalEvent>(store, Table::Events, &out.join("events.jsonl"))?,
        ),
        (
            "performances".to_string(),
            table_rows::<CanonicalPerformance>(
                store,
                Table::Performances,
                &out.join("performances.jsonl"),
            )?,
        ),
    ])
}

/// The derived planes are findings, not bulk: an operator reads the retained conflicts, the review
/// queue, the per-jurisdiction coverage, the snapshot history and the access conditions. The
/// source-identity table is deliberately absent — it is the join table behind those rows, one row
/// per canonical id per source, and dumping it would dwarf everything else here.
fn finding_counts(store: &Store, out: &Path) -> StoreResult<Vec<(String, usize)>> {
    Ok(vec![
        (
            "conflicts".to_string(),
            table_rows::<RetainedConflict>(store, Table::Conflicts, &out.join("conflicts.jsonl"))?,
        ),
        (
            "review_cases".to_string(),
            table_rows::<ReviewCase>(store, Table::ReviewCases, &out.join("review_cases.jsonl"))?,
        ),
        (
            "coverage".to_string(),
            table_rows::<CoverageRow>(store, Table::Coverage, &out.join("coverage.jsonl"))?,
        ),
        (
            "snapshots".to_string(),
            table_rows::<CollectionSnapshot>(
                store,
                Table::Snapshots,
                &out.join("snapshots.jsonl"),
            )?,
        ),
        (
            "identity_verdicts".to_string(),
            table_rows::<ReviewVerdictRecord>(
                store,
                Table::IdentityVerdicts,
                &out.join("identity_verdicts.jsonl"),
            )?,
        ),
        (
            "source_access".to_string(),
            table_rows::<SourceAccessCondition>(
                store,
                Table::SourceAccess,
                &out.join("source_access.jsonl"),
            )?,
        ),
    ])
}

/// The row count of one consolidated table, materialized as `path`.
fn table_rows<T: Entity>(store: &Store, table: Table, path: &Path) -> StoreResult<usize> {
    Ok(store.consolidate::<T>(table, path)?.rows)
}
