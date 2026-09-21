//! Aggregation: the per-state outcomes folded into one report, and the append logs into snapshots.
//!
//! `summarize_states` owns the report totals, the per-state row order and the failure list — the
//! caller keeps its journal and re-runs, so a partial walk is reported rather than discarded — and
//! `consolidate` merges the append logs into the `out/*.jsonl` read model, counting what it wrote.

use crate::model::{CanonicalAthlete, CanonicalSchool, CanonicalTeam};
use crate::store::{Store, Table};
use anyhow::Result;
use tracing::info;

use super::{CollectReport, StateProgress};

/// `usize` -> `u64` for the report counters, saturating where the value cannot fit.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Fold the per-state outcomes into one report, returning it with the states that failed.
///
/// Bounded by the number of requested states.
pub(super) fn summarize_states(
    results: Vec<(String, Result<StateProgress>)>,
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
    for (state, outcome) in results {
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
                    state,
                    teams = progress.teams,
                    rosters = progress.rosters_done,
                    co2027 = progress.class_of_2027,
                    "state complete"
                );
                report.states.push(progress);
            }
            Err(error) => failures.push(format!("{state}: {error:#}")),
        }
    }
    report
        .states
        .sort_by(|left, right| left.state.cmp(&right.state));
    (report, failures)
}

/// Merge append logs into snapshots under `out/`, returning per-table counts.
pub fn consolidate(store: &Store) -> Result<Vec<(String, usize)>> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let mut counts = Vec::new();
    counts.push((
        "schools".to_string(),
        store
            .consolidate::<CanonicalSchool>(Table::Schools, &out.join("schools.jsonl"))?
            .rows,
    ));
    counts.push((
        "teams".to_string(),
        store
            .consolidate::<CanonicalTeam>(Table::Teams, &out.join("teams.jsonl"))?
            .rows,
    ));
    let coaches_path = out.join("coaches.jsonl");
    let coaches =
        store.consolidate::<crate::model::CanonicalCoach>(Table::Coaches, &coaches_path)?;
    counts.push(("coaches".to_string(), coaches.rows));
    // The merge withholds consumer mailboxes before the snapshot is written, so this counts the
    // same rule the report and the workbook already went through.
    counts.push(("coaches_email_withheld".to_string(), coaches.withheld));
    counts.push((
        "athletes".to_string(),
        store
            .consolidate::<CanonicalAthlete>(Table::Athletes, &out.join("athletes.jsonl"))?
            .rows,
    ));
    counts.push((
        "meets".to_string(),
        store
            .consolidate::<crate::model::CanonicalMeet>(Table::Meets, &out.join("meets.jsonl"))?
            .rows,
    ));
    counts.push((
        "events".to_string(),
        store
            .consolidate::<crate::model::CanonicalEvent>(Table::Events, &out.join("events.jsonl"))?
            .rows,
    ));
    counts.push((
        "performances".to_string(),
        store
            .consolidate::<crate::model::CanonicalPerformance>(
                Table::Performances,
                &out.join("performances.jsonl"),
            )?
            .rows,
    ));
    Ok(counts)
}
