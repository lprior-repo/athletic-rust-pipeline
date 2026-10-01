use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, DataType, Reader};
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use sha2::{Digest, Sha256};

use crate::census::WorkbookCheck;

use super::count;

mod checks;
mod reconcile;
mod result;

pub const ATHLETES_SHEET: &str = "Athletes";
pub const COVERAGE_SHEET: &str = "Coverage";
pub const RUN_METRICS_SHEET: &str = "Run Metrics";

pub const ATHLETE_METRIC_LABEL: &str = "recruiting athletes";

pub fn inspect_workbook(
    path: &Path,
    dataset: &ExportDataset,
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
    let derivation = Derivation::of(dataset, scope, Some(grad_year));
    let expected_athletes = count(derivation.athletes().len());
    let coverage = report::coverage_report(dataset, Some(grad_year))
        .with_context(|| "reading the coverage report for workbook reconciliation")?;
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

pub fn labelled_count(rows: &[Vec<String>], label: &str) -> Option<u64> {
    let row = rows.iter().find(|row| {
        row.first()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(label))
    })?;
    row.get(1)?.trim().parse().ok()
}

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
