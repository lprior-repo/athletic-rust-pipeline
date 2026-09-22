//! The seal command's workbook checks, on workbooks this module writes and reads back.
//!
//! The reconciliation rules are the point: a workbook that disagrees with the store must refuse the
//! seal and name the number that disagreed, because a seal over an unverified export is worse than
//! no seal at all.

use super::workbook::{
    file_digest, inspect_workbook, labelled_count, ATHLETES_SHEET, COHORT_LABEL, COVERAGE_SHEET,
    RUN_METRICS_SHEET,
};
use super::*;
use rust_xlsxwriter::Workbook as Xlsx;

/// Write a workbook holding the three sheets the seal requires: `Athletes`, `Coverage` carrying
/// `jurisdictions` data rows, and `Run Metrics` naming the cohort when one is given.
fn workbook(dir: &Path, cohort: Option<u64>, jurisdictions: u32) -> Result<PathBuf> {
    let path = dir.join("midwest-census-test.xlsx");
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
        metrics.write_number(2, 1, f64::from(u32::try_from(count)?))?;
    }

    book.save(&path)?;
    Ok(path)
}

#[test]
fn a_workbook_that_agrees_with_the_store_verifies() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = workbook(dir.path(), Some(307_653), 51).expect("the workbook writes");

    let check = inspect_workbook(&path, 307_653, 51).expect("the workbook reads back");

    assert_eq!(check.mapped_athletes, 307_653);
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
    let path = workbook(dir.path(), Some(307_652), 51).expect("the workbook writes");

    let check = inspect_workbook(&path, 307_653, 51).expect("the workbook reads back");

    assert!(!check.counts_reconciled);
    assert!(!check.export_verified);
    assert_eq!(check.discrepancies.len(), 1);
    let named = &check.discrepancies[0];
    assert!(
        named.contains("307652") && named.contains("307653"),
        "{named}"
    );
}

#[test]
fn a_workbook_that_omits_the_cohort_row_refuses() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = workbook(dir.path(), None, 51).expect("the workbook writes");

    let check = inspect_workbook(&path, 307_653, 51).expect("the workbook reads back");

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
    let path = workbook(dir.path(), Some(307_653), 3).expect("the workbook writes");

    let check = inspect_workbook(&path, 307_653, 51).expect("the workbook reads back");

    assert!(!check.coverage_reconciled);
    assert!(check.counts_reconciled, "the cohort still agrees");
    assert!(!check.export_verified);
    assert_eq!(check.discrepancies.len(), 1);
}

#[test]
fn a_workbook_missing_a_required_sheet_refuses() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("athletes-only.xlsx");
    let mut book = Xlsx::new();
    book.add_worksheet()
        .set_name(ATHLETES_SHEET)
        .expect("naming a sheet")
        .write_string(0, 0, "athlete")
        .expect("a header");
    book.save(&path).expect("the workbook writes");

    let check = inspect_workbook(&path, 0, 0).expect("the workbook reads back");

    assert_eq!(check.sheets, 1);
    assert!(!check.export_verified);
    let named = check.discrepancies.join("; ");
    assert!(
        named.contains(COVERAGE_SHEET) && named.contains(RUN_METRICS_SHEET),
        "the refusal must name the sheets the workbook lacks: {named}"
    );
    assert!(!check.counts_reconciled && !check.metrics_reconciled);
}

#[test]
fn the_digest_moves_with_the_bytes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = workbook(dir.path(), Some(307_653), 51).expect("the workbook writes");
    let same = file_digest(&first).expect("hashes");
    assert_eq!(same, file_digest(&first).expect("hashes again"));

    let moved = dir.path().join("other.xlsx");
    std::fs::copy(&first, &moved).expect("copies");
    assert_eq!(same, file_digest(&moved).expect("hashes the copy"));

    std::fs::write(&moved, b"not a workbook").expect("overwrites");
    assert_ne!(same, file_digest(&moved).expect("hashes the change"));
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
fn stats_with(snapshots: u64, review: u64, coverage: u64, observations: u64) -> StoreStats {
    StoreStats {
        tables: vec![
            (Table::Snapshots.file().to_string(), snapshots),
            (Table::ReviewCases.file().to_string(), review),
            (Table::Coverage.file().to_string(), coverage),
        ],
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
    let path = workbook(dir.path(), Some(307_653), 51).expect("the workbook writes");
    let state = reached_phase(&stats_with(1, 2, 3, 9), &path).expect("walks");
    assert_eq!(
        state.phase(),
        Phase::Exporting,
        "only the export lifts the last step"
    );
}
