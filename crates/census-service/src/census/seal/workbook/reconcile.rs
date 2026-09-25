use std::collections::HashSet;

use super::{
    count, labelled_count, sheet_rows, ATHLETES_SHEET, COHORT_LABEL, COVERAGE_SHEET,
    RUN_METRICS_SHEET,
};

/// Result of coverage-sheet reconciliation.
pub(super) struct CoverageReconcile {
    pub(super) unique_count: usize,
    pub(super) has_duplicates: bool,
}

/// Result of run-metrics-sheet reconciliation.
pub(super) struct RunMetricsReconcile {
    pub(super) rows: Vec<Vec<String>>,
    pub(super) mapped_athletes: u64,
}

/// Read the coverage sheet, deduplicate jurisdictions, and reconcile against `expected_jurisdictions`.
///
/// Pushes a named discrepancy when rows are missing or counts diverge.
pub(super) fn reconcile_coverage(
    book: &mut calamine::Sheets<std::io::BufReader<super::File>>,
    discrepancies: &mut Vec<String>,
    expected_jurisdictions: u64,
) -> super::Result<CoverageReconcile> {
    match sheet_rows(book, COVERAGE_SHEET)? {
        Some(rows) => {
            let unique: HashSet<&str> = rows
                .iter()
                .skip(1)
                .filter_map(|row| row.first().map(|s| s.as_str()))
                .collect();
            let unique_count = unique.len();
            let data_rows = rows.len().saturating_sub(1);
            if unique_count < data_rows {
                let dupes = data_rows.saturating_sub(unique_count);
                discrepancies.push(format!(
                    "{COVERAGE_SHEET} lists {dupes} duplicate jurisdiction row(s); unique jurisdictions: {unique_count}"
                ));
            }
            if u64::try_from(unique_count).unwrap_or(u64::MAX) < expected_jurisdictions {
                discrepancies.push(format!(
                    "{COVERAGE_SHEET} carries {unique_count} unique jurisdiction rows, the store has {expected_jurisdictions}"
                ));
            }
            Ok(CoverageReconcile {
                unique_count,
                has_duplicates: unique_count < rows.len().saturating_sub(1),
            })
        }
        None => {
            discrepancies.push(format!(
                "{COVERAGE_SHEET} is missing, expected {expected_jurisdictions} jurisdictions"
            ));
            Ok(CoverageReconcile {
                unique_count: 0,
                has_duplicates: false,
            })
        }
    }
}

/// Count the data rows in the athletes sheet and reconcile against `expected_athletes`.
///
/// Pushes a named discrepancy when rows are missing or counts diverge.
pub(super) fn reconcile_athletes(
    book: &mut calamine::Sheets<std::io::BufReader<super::File>>,
    discrepancies: &mut Vec<String>,
    expected_athletes: u64,
) -> super::Result<u64> {
    match sheet_rows(book, ATHLETES_SHEET)? {
        Some(rows) => {
            let data_rows = rows.iter().filter(|r| !r.is_empty()).count();
            let actual_count = count(data_rows.saturating_sub(1)); // skip header
            if actual_count == 0 {
                discrepancies.push(format!(
                    "{ATHLETES_SHEET} has no athlete data rows; the store has {expected_athletes}"
                ));
            } else if actual_count != expected_athletes {
                discrepancies.push(format!(
                    "{ATHLETES_SHEET} carries {actual_count} athlete rows; the store has {expected_athletes}"
                ));
            }
            Ok(actual_count)
        }
        None => {
            discrepancies.push(format!(
                "{ATHLETES_SHEET} is missing; expected {expected_athletes} athletes"
            ));
            Ok(0)
        }
    }
}

/// Read the run-metrics sheet, map the cohort label to a count, and reconcile.
///
/// Pushes a named discrepancy when the cohort row is absent or the count diverges.
pub(super) fn reconcile_run_metrics(
    book: &mut calamine::Sheets<std::io::BufReader<super::File>>,
    discrepancies: &mut Vec<String>,
    expected_athletes: u64,
) -> super::Result<RunMetricsReconcile> {
    let run_metrics = sheet_rows(book, RUN_METRICS_SHEET)?.unwrap_or_default();
    let mapped_athletes = labelled_count(&run_metrics, COHORT_LABEL).unwrap_or(0);
    if mapped_athletes == 0 {
        discrepancies.push(format!(
            "{RUN_METRICS_SHEET} does not name the cohort: no {COHORT_LABEL} row"
        ));
    } else if mapped_athletes != expected_athletes {
        discrepancies.push(format!(
            "{RUN_METRICS_SHEET} cohort {mapped_athletes} != store {expected_athletes}"
        ));
    }
    Ok(RunMetricsReconcile {
        rows: run_metrics,
        mapped_athletes,
    })
}
