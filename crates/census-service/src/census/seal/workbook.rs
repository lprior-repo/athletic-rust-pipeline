//! Reading the exported workbook back: what the seal can honestly check about the bytes it certifies.
//!
//! Every count the seal verifies is read from the store — not handed through by a caller that
//! could lie — and every mismatch is a named discrepancy rather than a silent pass.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use census_report::report::{self, Scope};
use census_store::Store;
use sha2::{Digest, Sha256};

use crate::census::WorkbookCheck;

use super::count;

mod checks;
mod reconcile;
mod result;

/// The three sheet names the seal requires. They are what the workbook's meta block publishes; a
/// rename shows up here as a refusal rather than as a seal over an unverified export.
pub const ATHLETES_SHEET: &str = "Athletes";
pub const COVERAGE_SHEET: &str = "Coverage";
pub const RUN_METRICS_SHEET: &str = "Run Metrics";

/// The run-metrics label whose value must equal the store's cohort count.
pub const COHORT_LABEL: &str = "class of 2027";

/// Verify an exported workbook against the store's own counts.
///
/// The seal never trusts a caller's tally. It reads the census and coverage report from the store
/// to get the expected athlete count and jurisdiction count, then checks the workbook's actual rows
/// against those expectations. Every mismatch is a named discrepancy.
pub fn inspect_workbook(
    path: &Path,
    store: &Store,
    grad_year: i16,
    scope: Scope,
) -> Result<WorkbookCheck> {
    let digest = file_digest(path)?;
    let mut book =
        open_workbook_auto(path).with_context(|| format!("opening {}", path.display()))?;
    let sheet_names = book.sheet_names();
    let names: Vec<&str> = sheet_names.iter().map(|name| name.as_str()).collect();
    let mut discrepancies = Vec::new();
    discrepancies.extend(checks::check_required_sheets(
        &names,
        &[ATHLETES_SHEET, COVERAGE_SHEET, RUN_METRICS_SHEET],
    ));
    let census = report::build_census(store, scope)
        .with_context(|| "reading the census from the store for workbook reconciliation")?;
    let expected_athletes = count(census.totals.class_of_2027);
    let coverage = report::coverage_report(store, Some(grad_year)).with_context(|| {
        "reading the coverage report from the store for workbook reconciliation"
    })?;
    let expected_jurisdictions = count(coverage.jurisdictions.len());
    let cov = reconcile::reconcile_coverage(&mut book, &mut discrepancies, expected_jurisdictions)?;
    let athletes_data_rows =
        reconcile::reconcile_athletes(&mut book, &mut discrepancies, expected_athletes)?;
    let reconciled =
        reconcile::reconcile_run_metrics(&mut book, &mut discrepancies, expected_athletes)?;
    let run_metrics = reconciled.rows;
    let mapped_athletes = reconciled.mapped_athletes;
    let counts_reconciled =
        athletes_data_rows == expected_athletes && mapped_athletes == expected_athletes;
    let expected_juris_usize = usize::try_from(expected_jurisdictions).unwrap_or(usize::MAX);
    let coverage_reconciled = cov.unique_count == expected_juris_usize && !cov.has_duplicates;
    let metrics_reconciled = !run_metrics.is_empty() && mapped_athletes > 0;
    let export_verified = discrepancies.is_empty();
    Ok(result::build_check(result::CheckAssembly {
        names: &names,
        coverage_unique_count: cov.unique_count,
        run_metrics: &run_metrics,
        digest,
        mapped_athletes,
        decisions: result::CheckDecisions {
            counts_reconciled,
            coverage_reconciled,
            metrics_reconciled,
            export_verified,
        },
        discrepancies,
    }))
}

/// Every non-empty row of one sheet, each row as text cells.
fn sheet_rows(
    book: &mut calamine::Sheets<std::io::BufReader<File>>,
    name: &str,
) -> Result<Option<Vec<Vec<String>>>> {
    if !book.sheet_names().iter().any(|sheet| sheet == name) {
        return Ok(None);
    }
    let range = book
        .worksheet_range(name)
        .with_context(|| format!("reading sheet {name}"))?;
    let rows = range
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| cell.as_string().unwrap_or_default())
                .collect()
        })
        .collect();
    Ok(Some(rows))
}

/// The number a run-metrics row carries for `label`, when the label is written as the row's first
/// cell and the count as any later cell.
///
/// The label match is case-insensitive because the sheet's own casing is the workbook writer's
/// choice, not this command's contract.
///
/// The Run Metrics sheet renders two scope columns side by side (`Core` / `All sources`), each
/// carrying its own copy of the label.  When the function finds such a row it must pick the
/// *second* numeric cell (the All‑sources column) so the seal compares the same scope the store
/// published — the Core count is always a subset and will never equal the store's total.
pub fn labelled_count(rows: &[Vec<String>], label: &str) -> Option<u64> {
    rows.iter().find_map(|row| {
        let named = row.first()?.trim().to_ascii_lowercase();
        if !named.starts_with(label) {
            return None;
        }
        let mut candidates: Vec<u64> = row
            .iter()
            .skip(1)
            .filter_map(|cell| {
                let digits: String = cell.chars().filter(|ch| ch.is_ascii_digit()).collect();
                if digits.is_empty() {
                    None
                } else {
                    digits.parse::<u64>().ok()
                }
            })
            .collect();
        candidates.pop()
    })
}

/// The workbook's own bytes, hashed in chunks so a large export never lands in memory twice.
pub fn file_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("reading {}", path.display()))?;
        if read == 0 {
            break;
        }
        let chunk = buffer
            .get(..read)
            .context("a digest read cannot exceed its buffer")?;
        hasher.update(chunk);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
