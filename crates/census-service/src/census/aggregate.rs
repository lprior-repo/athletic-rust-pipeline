
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

use super::{CollectReport, StateProgress, TransportReport};

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(super) fn summarize_states(
    results: Vec<(UsJurisdiction, CrawlResult<StateProgress>)>,
) -> (CollectReport, Vec<String>) {
    let mut report = CollectReport {
        states: Vec::new(),
        teams_total: 0,
        rosters_fetched: 0,
        athletes_total: 0,
        class_of_2027_total: 0,
        errors: 0,
        elapsed_seconds: 0.0,
        transport: TransportReport::default(),
        access_conditions: Vec::new(),
        blocked_hosts: Vec::new(),
    };
    let mut failures = Vec::new();
    for (jurisdiction, outcome) in results {
        match outcome {
            Ok(progress) => {
                let new_teams = report.teams_total.saturating_add(progress.rosters_total);
                let new_fetched = report.rosters_fetched.saturating_add(progress.rosters_committed);
                let new_co2027 = report.class_of_2027_total.saturating_add(progress.class_of_2027);
                let new_errors = report.errors.saturating_add(count(progress.errors.len()));
                let new_athletes = report.athletes_total.saturating_add(progress.athletes);
                report.teams_total = new_teams;
                report.rosters_fetched = new_fetched;
                report.class_of_2027_total = new_co2027;
                report.errors = new_errors;
                report.athletes_total = new_athletes;
                info!(
                    state = jurisdiction.code(),
                    teams = progress.rosters_total,
                    co2027 = progress.class_of_2027,
                    rosters = progress.rosters_committed,
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

fn table_rows<T: Entity>(store: &Store, table: Table, path: &Path) -> StoreResult<usize> {
    Ok(store.consolidate::<T>(table, path)?.rows)
}
