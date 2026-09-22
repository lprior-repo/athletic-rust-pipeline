//! Aggregation: the per-state outcomes folded into one report, and the append logs into snapshots.
//!
//! `summarize_states` owns the report totals, the per-state row order and the failure list — the
//! caller keeps its journal and re-runs, so a partial walk is reported rather than discarded — and
//! `consolidate` merges the append logs into the `out/*.jsonl` read model, counting what it wrote.

use crate::sources::CrawlResult;
use crate::store::{Entity, Store, StoreError, StoreResult, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use census_domain::UsJurisdiction;
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
    let mut counts = Vec::new();
    counts.push((
        "schools".to_string(),
        table_rows::<CanonicalSchool>(store, Table::Schools, &out.join("schools.jsonl"))?,
    ));
    counts.push((
        "teams".to_string(),
        table_rows::<CanonicalTeam>(store, Table::Teams, &out.join("teams.jsonl"))?,
    ));
    let coaches_path = out.join("coaches.jsonl");
    let coaches = store.consolidate::<CanonicalCoach>(Table::Coaches, &coaches_path)?;
    counts.push(("coaches".to_string(), coaches.rows));
    // The merge withholds consumer mailboxes before the snapshot is written, so this counts the
    // same rule the report and the workbook already went through.
    counts.push(("coaches_email_withheld".to_string(), coaches.withheld));
    counts.push((
        "athletes".to_string(),
        table_rows::<CanonicalAthlete>(store, Table::Athletes, &out.join("athletes.jsonl"))?,
    ));
    counts.push((
        "meets".to_string(),
        table_rows::<CanonicalMeet>(store, Table::Meets, &out.join("meets.jsonl"))?,
    ));
    counts.push((
        "events".to_string(),
        table_rows::<CanonicalEvent>(store, Table::Events, &out.join("events.jsonl"))?,
    ));
    counts.push((
        "performances".to_string(),
        table_rows::<CanonicalPerformance>(
            store,
            Table::Performances,
            &out.join("performances.jsonl"),
        )?,
    ));
    Ok(counts)
}

/// The row count of one consolidated table, materialized as `path`.
fn table_rows<T: Entity>(store: &Store, table: Table, path: &Path) -> StoreResult<usize> {
    Ok(store.consolidate::<T>(table, path)?.rows)
}
