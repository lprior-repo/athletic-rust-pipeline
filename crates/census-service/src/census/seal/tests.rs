//! The seal command's workbook checks, on workbooks this module writes and reads back.
//!
//! The reconciliation rules are the point: a workbook that disagrees with the store must refuse the
//! seal and name the number that disagreed, because a seal over an unverified export is worse than
//! no seal at all.

use std::path::{Path, PathBuf};

use anyhow::Result;
use census_domain::model::{
    AccessBlockKind, CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool,
    CompetitionLevel, Gender, GradYear, SourceAccessCondition,
};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_store::{Store, StoreStats, Table};
use rust_xlsxwriter::Workbook as Xlsx;

use super::reached_phase;
use super::retained_access;
use super::workbook::{
    file_digest, inspect_workbook, labelled_count, ATHLETES_SHEET, COHORT_LABEL, COVERAGE_SHEET,
    RUN_METRICS_SHEET,
};
use crate::census::Phase;

/// The coverage report always publishes rows for all jurisdictions in CENSUS_SCOPE (49) plus
/// the unplaced row, so the expected jurisdiction count is always 50 regardless of seeded data.
const EXPECTED_JURISDICTIONS: u32 = 50;

/// All US jurisdictions in census-scope order (ADR-009) — 49 contiguous states + DC.
fn all_jurisdictions() -> [UsJurisdiction; 51] {
    [
        UsJurisdiction::Alabama,
        UsJurisdiction::Alaska,
        UsJurisdiction::Arizona,
        UsJurisdiction::Arkansas,
        UsJurisdiction::California,
        UsJurisdiction::Colorado,
        UsJurisdiction::Connecticut,
        UsJurisdiction::Delaware,
        UsJurisdiction::Florida,
        UsJurisdiction::Georgia,
        UsJurisdiction::Hawaii,
        UsJurisdiction::Idaho,
        UsJurisdiction::Illinois,
        UsJurisdiction::Indiana,
        UsJurisdiction::Iowa,
        UsJurisdiction::Kansas,
        UsJurisdiction::Kentucky,
        UsJurisdiction::Louisiana,
        UsJurisdiction::Maine,
        UsJurisdiction::Maryland,
        UsJurisdiction::Massachusetts,
        UsJurisdiction::Michigan,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Mississippi,
        UsJurisdiction::Missouri,
        UsJurisdiction::Montana,
        UsJurisdiction::Nebraska,
        UsJurisdiction::Nevada,
        UsJurisdiction::NewHampshire,
        UsJurisdiction::NewJersey,
        UsJurisdiction::NewMexico,
        UsJurisdiction::NewYork,
        UsJurisdiction::NorthCarolina,
        UsJurisdiction::NorthDakota,
        UsJurisdiction::Ohio,
        UsJurisdiction::Oklahoma,
        UsJurisdiction::Oregon,
        UsJurisdiction::Pennsylvania,
        UsJurisdiction::RhodeIsland,
        UsJurisdiction::SouthCarolina,
        UsJurisdiction::SouthDakota,
        UsJurisdiction::Tennessee,
        UsJurisdiction::Texas,
        UsJurisdiction::Utah,
        UsJurisdiction::Vermont,
        UsJurisdiction::Virginia,
        UsJurisdiction::Washington,
        UsJurisdiction::WestVirginia,
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Wyoming,
        UsJurisdiction::DistrictOfColumbia,
    ]
}

/// Write workbooks with all 50 unique jurisdictions in the coverage sheet.
fn write_coverage_sheets(
    book: &mut Xlsx,
    n_jurisdictions: u32,
    dupes_per_jurisdiction: u32,
) -> Result<()> {
    let coverage = book.add_worksheet().set_name(COVERAGE_SHEET)?;
    coverage.write_string(0, 0, "state")?;
    let states = all_jurisdictions();
    // Only the first 49 are in CENSUS_SCOPE (Alaska=1, Hawaii=11 are excluded)
    let scope_states = &states[..50];
    let mut row = 1u32;
    for i in 0..n_jurisdictions {
        let state = scope_states[i as usize % scope_states.len()];
        for d in 0..dupes_per_jurisdiction {
            coverage.write_string(row + d, 0, state.code())?;
        }
        row += dupes_per_jurisdiction;
    }
    Ok(())
}

/// Seed a store with schools, athletes (of a specific grad year), coaches, meets, and coverage.
/// Returns the store and the path where the workbook was written.
/// `athlete_count` is the number of class_of_2027 athletes; the workbook will have 50 unique
/// jurisdictions in Coverage and `cohort` in Run Metrics.
fn seed_and_workbook(
    dir: &Path,
    athlete_count: u32,
    cohort: Option<u64>,
) -> Result<(Store, PathBuf)> {
    let store = Store::open(dir)?;

    // Create schools in all 49 CENSUS_SCOPE jurisdictions.
    // The coverage report reads jurisdictions from schools, athletes, coaches, meets.
    // We need schools in every jurisdiction for the coverage report to see all of them.
    let scope_states: &[UsJurisdiction; 49] = &all_jurisdictions()
        .iter()
        .filter(|s| **s != UsJurisdiction::Alaska && **s != UsJurisdiction::Hawaii)
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap(); // First 50 minus Alaska(1) and Hawaii(11)

    for (i, state) in scope_states.iter().enumerate() {
        let (school, _) = CanonicalSchool::new(
            *state,
            format!("Test High School {i}"),
            format!("testhighschool{i}"),
        );
        store.append(Table::Schools, &school)?;
    }

    // Create athletes with grad_year = 2027, distributed across schools.
    for i in 0..athlete_count {
        let school_idx = i as usize % scope_states.len();
        let school = &scope_states[school_idx];
        let (school, school_id) = CanonicalSchool::new(
            *school,
            format!("Athlete School {i}"),
            format!("athleteschool{i}"),
        );
        store.append(Table::Schools, &school)?;
        let athlete = CanonicalAthlete::new(
            &school_id,
            format!("Athlete {i}"),
            GradYear::new(2027).unwrap(),
            if i % 2 == 0 {
                Gender::Boys
            } else {
                Gender::Girls
            },
        );
        store.append(Table::Athletes, &athlete)?;
    }

    // Create one coach and one meet per school.
    for (i, state) in scope_states.iter().enumerate() {
        let (school, school_id) = CanonicalSchool::new(
            *state,
            format!("Coach School {i}"),
            format!("coachschool{i}"),
        );
        store.append(Table::Schools, &school)?;
        let coach = CanonicalCoach::new(
            &school_id,
            format!("Coach {i}"),
            None,
            Gender::Boys,
            census_domain::model::CoachRole::HeadCoach,
        );
        store.append(Table::Coaches, &coach)?;
        let meet = CanonicalMeet::new(
            Some(*state),
            format!("Meet {i}"),
            "2025-05-01",
            CompetitionLevel::Invitational,
        );
        store.append(Table::Meets, &meet)?;
    }

    // Write the workbook with 50 unique jurisdictions.
    let path = dir.join("census-service-test.xlsx");
    let mut book = Xlsx::new();

    // Athletes sheet.
    let athletes = book.add_worksheet().set_name(ATHLETES_SHEET)?;
    athletes.write_string(0, 0, "athlete")?;
    for i in 0..athlete_count {
        athletes.write_string(i + 1, 0, format!("Athlete {i}"))?;
    }

    // Coverage sheet with unique jurisdictions.
    write_coverage_sheets(&mut book, EXPECTED_JURISDICTIONS, 1)?;

    // Run Metrics sheet.
    let metrics = book.add_worksheet().set_name(RUN_METRICS_SHEET)?;
    metrics.write_string(0, 0, "metric")?;
    metrics.write_string(0, 1, "value")?;
    metrics.write_string(1, 0, "Athletes")?;
    if let Some(count) = cohort {
        metrics.write_string(2, 0, "Class of 2027")?;
        metrics.write_number(2, 1, count as f64)?;
    }

    book.save(&path)?;

    Ok((store, path))
}

/// Write a simple workbook: header-only Athletes, `jurisdictions` copies of "WI" in Coverage,
/// and Run Metrics with the given cohort value.
fn simple_workbook(dir: &Path, cohort: Option<u64>, jurisdictions: u32) -> Result<PathBuf> {
    let path = dir.join("census-service-test.xlsx");
    let mut book = Xlsx::new();

    let athletes = book.add_worksheet().set_name(ATHLETES_SHEET)?;
    athletes.write_string(0, 0, "athlete")?;

    let coverage = book.add_worksheet().set_name(COVERAGE_SHEET)?;
    coverage.write_string(0, 0, "state")?;
    for row in 0..jurisdictions {
        coverage.write_string(row + 1, 0, "WI")?;
    }

    let metrics = book.add_worksheet().set_name(RUN_METRICS_SHEET)?;
    metrics.write_string(0, 0, "metric")?;
    metrics.write_string(0, 1, "value")?;
    metrics.write_string(1, 0, "Athletes")?;
    if let Some(count) = cohort {
        metrics.write_string(2, 0, "Class of 2027")?;
        metrics.write_number(2, 1, count as f64)?;
    }

    book.save(&path)?;
    Ok(path)
}

/// Write a workbook with duplicated jurisdictions: the first jurisdiction appears `dupe_count` times,
/// then `n_unique` additional unique jurisdictions.
fn workbook_with_dupes(
    dir: &Path,
    cohort: Option<u64>,
    n_unique: u32,
    dupe_count: u32,
) -> Result<PathBuf> {
    let path = dir.join("census-service-test.xlsx");
    let mut book = Xlsx::new();

    let athletes = book.add_worksheet().set_name(ATHLETES_SHEET)?;
    athletes.write_string(0, 0, "athlete")?;

    let coverage = book.add_worksheet().set_name(COVERAGE_SHEET)?;
    coverage.write_string(0, 0, "state")?;
    // First jurisdiction duplicated `dupe_count` times
    for i in 0..dupe_count {
        coverage.write_string(i + 1, 0, "WI")?;
    }
    // Then unique jurisdictions
    let states = [
        "AK", "AZ", "CA", "CO", "CT", "FL", "GA", "IL", "MA", "MD", "MI", "MN", "NY", "NC", "OH",
        "OR", "PA", "TX", "WA", "WI",
    ];
    for i in 0..n_unique {
        coverage.write_string(dupe_count + 1 + i, 0, states[i as usize % states.len()])?;
    }

    let metrics = book.add_worksheet().set_name(RUN_METRICS_SHEET)?;
    metrics.write_string(0, 0, "metric")?;
    metrics.write_string(0, 1, "value")?;
    metrics.write_string(1, 0, "Athletes")?;
    if let Some(count) = cohort {
        metrics.write_string(2, 0, "Class of 2027")?;
        metrics.write_number(2, 1, count as f64)?;
    }

    book.save(&path)?;
    Ok(path)
}

#[test]
fn a_zero_athlete_workbook_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (store, path) = seed_and_workbook(dir.path(), 0, Some(0)).expect("seeds");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert!(!check.export_verified, "zero athletes must be refused");
    assert!(
        check.discrepancies.iter().any(|d| d.contains("Athletes")),
        "discrepancy must name the Athletes sheet: {:?}",
        check.discrepancies
    );
}

#[test]
fn a_workbook_that_agrees_with_the_store_verifies() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let athlete_count: u32 = 5;
    let (store, path) = seed_and_workbook(dir.path(), athlete_count, Some(5)).expect("seeds");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert_eq!(check.mapped_athletes, 5);
    assert!(
        check.counts_reconciled,
        "discrepancies: {:?}",
        check.discrepancies
    );
    assert!(check.coverage_reconciled);
    assert!(check.metrics_reconciled);
    assert!(
        check.export_verified,
        "discrepancies: {:?}",
        check.discrepancies
    );
    assert_eq!(check.discrepancies, Vec::<String>::new());
    assert_eq!(check.sheets, 3);
    assert_eq!(check.digests.len(), 1);
    assert_eq!(check.digests[0].len(), 64, "sha256 hex");
}

#[test]
fn a_workbook_that_disagrees_refuses_and_names_the_number() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Store has 5 athletes, workbook says 4
    let (store, path) = seed_and_workbook(dir.path(), 5, Some(4)).expect("seeds");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert!(!check.counts_reconciled);
    assert!(!check.export_verified);
    let named = &check.discrepancies[0];
    // The Run Metrics sheet says 4, but the store has 5
    assert!(named.contains("4") && named.contains("5"), "{named}");
}

#[test]
fn a_workbook_that_omits_the_cohort_row_refuses() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (store, path) = seed_and_workbook(dir.path(), 5, None).expect("seeds");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert_eq!(check.mapped_athletes, 0);
    assert!(!check.counts_reconciled);
    assert!(
        !check.metrics_reconciled,
        "a metrics sheet without the cohort is not a metric"
    );
    assert!(!check.export_verified);
}

#[test]
fn a_coverage_sheet_short_of_jurisdictions_refuses() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Store has 50 jurisdictions (coverage report), workbook only has 3
    let (store, _) = seed_and_workbook(dir.path(), 5, Some(5)).expect("seeds");

    // Write a workbook with only 3 coverage rows (override the seeded one)
    let path = simple_workbook(dir.path(), Some(5), 3).expect("writes override");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert!(!check.coverage_reconciled);
    assert!(!check.export_verified);
}

#[test]
fn a_workbook_missing_a_required_sheet_refuses() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let store = Store::open(dir.path()).expect("open temp store");
    let path = dir.path().join("athletes-only.xlsx");
    let mut book = Xlsx::new();
    book.add_worksheet()
        .set_name(ATHLETES_SHEET)
        .expect("naming a sheet")
        .write_string(0, 0, "athlete")
        .expect("a header");
    book.save(&path).expect("the workbook writes");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert_eq!(check.sheets, 1);
    assert!(!check.export_verified);
    let named = check.discrepancies.join("; ");
    assert!(
        named.contains(COVERAGE_SHEET) && named.contains(RUN_METRICS_SHEET),
        "the refusal must name the sheets the workbook lacks: {named}"
    );
}

#[test]
fn the_digest_moves_with_the_bytes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = simple_workbook(dir.path(), Some(5), 3).expect("writes first");
    let same = file_digest(&first).expect("hashes");
    assert_eq!(same, file_digest(&first).expect("hashes again"));

    let moved = dir.path().join("other.xlsx");
    std::fs::copy(&first, &moved).expect("copies");
    assert_eq!(same, file_digest(&moved).expect("hashes the copy"));

    std::fs::write(&moved, b"not a workbook").expect("overwrites");
    assert_ne!(same, file_digest(&moved).expect("hashes the change"));
}

#[test]
fn a_duplicated_coverage_jurisdiction_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Store has 50 jurisdictions
    let (store, _) = seed_and_workbook(dir.path(), 1, Some(1)).expect("seeds");

    // Write a workbook with WI duplicated 3 times (3 rows, 1 unique) plus 1 unique
    // Total: 4 rows, 2 unique — unique count (2) < expected (50), plus duplicates
    let path = workbook_with_dupes(dir.path(), Some(1), 1, 3).expect("writes workbook");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert!(!check.coverage_reconciled);
    assert!(!check.export_verified);
    let named = check.discrepancies.join("; ");
    assert!(
        named.contains("duplicate"),
        "discrepancy must name duplicates: {named}"
    );
}

#[test]
fn a_correct_workbook_still_verifies_with_unique_coverage() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Store has data, workbook has 50 unique jurisdictions matching expected
    let (store, path) = seed_and_workbook(dir.path(), 3, Some(3)).expect("seeds");

    let check = inspect_workbook(&path, &store, 2027, Scope::AllSources).expect("reads back");

    assert!(check.export_verified, "correct workbook should verify");
    assert!(check.coverage_reconciled);
    assert!(check.counts_reconciled);
}

#[test]
fn the_label_match_ignores_case_and_reads_a_grouped_number() {
    let rows = vec![
        vec!["Athletes".to_string(), "1,226,212".to_string()],
        vec!["Class of 2027".to_string(), String::new()],
        vec!["CLASS OF 2027".to_string(), "307,653".to_string()],
    ];
    assert_eq!(labelled_count(&rows, COHORT_LABEL), Some(307_653));
    assert_eq!(labelled_count(&rows, "schools"), None);
    assert_eq!(
        labelled_count(&rows[..2], COHORT_LABEL),
        None,
        "a label with no count names nothing"
    );
    assert_eq!(labelled_count(&[], COHORT_LABEL), None);
}

/// Store stats naming the artifacts the ladder's steps require.
///
/// `appended` stays empty: the ladder reads the row counts and the running observation total, not
/// the per-table sequence pointers, so the fixture names only the figures under test.
fn stats_with(snapshots: u64, review: u64, coverage: u64, observations: u64) -> StoreStats {
    StoreStats {
        tables: vec![
            (Table::Snapshots.file().to_string(), snapshots),
            (Table::ReviewCases.file().to_string(), review),
            (Table::Coverage.file().to_string(), coverage),
        ],
        appended: Vec::new(),
        observations,
        bytes_on_disk: 0,
        store_bytes: 0,
    }
}

#[test]
fn the_ladder_stops_at_the_first_artifact_the_store_lacks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let absent = dir.path().join("no-workbook.xlsx");
    let phases = [
        (stats_with(0, 0, 0, 0), Phase::Discovering),
        (stats_with(1, 0, 0, 9), Phase::Reconciling),
        (stats_with(1, 2, 0, 9), Phase::Reviewing),
        (stats_with(1, 2, 3, 9), Phase::ResolvingGaps),
    ];
    for (stats, expected) in phases {
        let state = reached_phase(&stats, &absent).expect("walks");
        assert_eq!(state.phase(), expected);
        assert!(state.sealed().is_none(), "a walked ladder is not sealed");
    }
}

#[test]
fn a_workbook_is_what_lifts_the_ladder_into_exporting() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = simple_workbook(dir.path(), Some(5), 3).expect("writes workbook");
    let state = reached_phase(&stats_with(1, 2, 3, 9), &path).expect("walks");
    assert_eq!(
        state.phase(),
        Phase::Exporting,
        "only the export lifts the last step"
    );
}

/// The access conditions the seal publishes are the store's own rows, split by what each kind means
/// for the lane: a refusal ends the work the host was asked for, a throttle is bounded by a cooldown.
/// The browser lane's two kinds are both here: a profile that wants a person is a refusal (no cooldown
/// ends it), while a machine with no lane is bounded by the lane's cooldown like any throttle.
#[test]
fn access_conditions_split_into_refusals_and_throttles() {
    let rows = vec![
        condition(AccessBlockKind::Forbidden, "refused.test"),
        condition(AccessBlockKind::RobotsDisallowed, "disallowed.test"),
        condition(AccessBlockKind::HumanRequired, "profile.test"),
        condition(AccessBlockKind::RateLimited, "throttled.test"),
        condition(AccessBlockKind::Timeout, "slow.test"),
        condition(AccessBlockKind::Unavailable, "down.test"),
        condition(AccessBlockKind::BrowserUnavailable, "lane.test"),
    ];

    assert_eq!(
        retained_access(&rows),
        (7, 3, 4),
        "three refusals and four cooldown-bounded rows over seven"
    );
    assert_eq!(
        retained_access(&[]),
        (0, 0, 0),
        "a store that has never been blocked retains nothing"
    );
}

/// One retained access condition of the given kind.
fn condition(kind: AccessBlockKind, host: &str) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "milesplit",
        host,
        kind,
        403,
        "2026-09-22T00:00:00Z",
        "probe",
    )
}
