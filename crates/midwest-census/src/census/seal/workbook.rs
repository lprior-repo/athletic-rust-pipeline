//! Reading the exported workbook back: what the seal can honestly check about the bytes it certifies.
//!
//! Sheet *contents* beyond the two meta sheets are deliberately not materialised — a census workbook
//! holds a million rows per sheet, and reading them to count cells would cost more than the
//! verification is worth. What costs nothing to prove is proved here: the file opens, it is the file
//! whose bytes were hashed, and the numbers its meta sheets publish agree with the store.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use sha2::{Digest, Sha256};

use crate::census::WorkbookCheck;

use super::count;

/// The three sheet names the seal requires. They are what the workbook's meta block publishes; a
/// rename shows up here as a refusal rather than as a seal over an unverified export.
pub const ATHLETES_SHEET: &str = "Athletes";
pub const COVERAGE_SHEET: &str = "Coverage";
pub const RUN_METRICS_SHEET: &str = "Run Metrics";

/// The run-metrics label whose value must equal the store's cohort count.
pub const COHORT_LABEL: &str = "class of 2027";

/// What the seal can honestly check about an exported workbook: its bytes, its sheet names, the
/// jurisdictions its coverage sheet carries, and the cohort its run-metrics sheet names.
///
/// Sheet *contents* beyond the two meta sheets are deliberately not materialised: a census workbook
/// holds a million rows per sheet, and reading them to count cells would cost more than the
/// verification is worth. What it costs nothing to prove — that the file is the one the store's
/// report describes — is proved here, and a mismatch is a named discrepancy rather than a refusal
/// with no reason.
pub fn inspect_workbook(
    path: &Path,
    class_of_2027: u64,
    jurisdictions: u64,
) -> Result<WorkbookCheck> {
    let digest = file_digest(path)?;
    let mut book =
        open_workbook_auto(path).with_context(|| format!("opening {}", path.display()))?;
    let names = book.sheet_names().to_vec();
    let mut discrepancies = Vec::new();

    for required in [ATHLETES_SHEET, COVERAGE_SHEET, RUN_METRICS_SHEET] {
        if !names.iter().any(|name| name == required) {
            discrepancies.push(format!("the workbook has no {required} sheet"));
        }
    }

    let coverage_rows = match sheet_rows(&mut book, COVERAGE_SHEET)? {
        Some(rows) => rows.len().saturating_sub(1),
        None => 0,
    };
    if count(coverage_rows) < jurisdictions {
        discrepancies.push(format!(
            "{COVERAGE_SHEET} carries {coverage_rows} jurisdiction rows, the classifier produced {jurisdictions}"
        ));
    }

    let run_metrics = sheet_rows(&mut book, RUN_METRICS_SHEET)?.unwrap_or_default();
    let mapped_athletes = labelled_count(&run_metrics, COHORT_LABEL).unwrap_or(0);
    if mapped_athletes == 0 {
        discrepancies.push(format!(
            "{RUN_METRICS_SHEET} does not name the cohort: no {COHORT_LABEL} row"
        ));
    } else if mapped_athletes != class_of_2027 {
        discrepancies.push(format!(
            "{RUN_METRICS_SHEET} cohort {mapped_athletes} != store {class_of_2027}"
        ));
    }

    let counts_reconciled = mapped_athletes > 0 && mapped_athletes == class_of_2027;
    let coverage_reconciled = count(coverage_rows) >= jurisdictions;
    let metrics_reconciled = !run_metrics.is_empty() && mapped_athletes > 0;
    let export_verified = discrepancies.is_empty();

    Ok(WorkbookCheck {
        sheets: count(names.len()),
        // See this function's doc: the big sheets are not materialised to count their cells, so the
        // row count is what the two meta sheets published — the rows this command actually read.
        rows: count(coverage_rows.saturating_add(run_metrics.len().saturating_sub(1))),
        digests: vec![digest],
        mapped_athletes,
        counts_reconciled,
        coverage_reconciled,
        metrics_reconciled,
        export_verified,
        discrepancies,
    })
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
pub fn labelled_count(rows: &[Vec<String>], label: &str) -> Option<u64> {
    rows.iter().find_map(|row| {
        let named = row.first()?.trim().to_ascii_lowercase();
        if !named.starts_with(label) {
            return None;
        }
        row.iter().skip(1).find_map(|cell| {
            let digits: String = cell.chars().filter(|ch| ch.is_ascii_digit()).collect();
            if digits.is_empty() {
                None
            } else {
                digits.parse::<u64>().ok()
            }
        })
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
