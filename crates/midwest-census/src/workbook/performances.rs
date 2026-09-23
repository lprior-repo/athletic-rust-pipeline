//! The `Performances_00N` sheets: every performance the census holds, one row per mark.
//!
//! The objective (§52) asks the recruiting workbook for *every available performance*, which is more
//! rows than one Excel worksheet holds: a sheet is capped at 1,048,576 rows. The rows are therefore
//! partitioned into `Performances_001`, `Performances_002`, … — every sheet repeats the column header
//! and carries at most [`DATA_ROWS_PER_SHEET`] data rows, which is Excel's cap minus the header minus
//! a [`PARTITION_MARGIN`]-row margin. A row is never truncated: a budget that cannot hold one is a
//! rejection that names it, not a quietly shorter sheet.
//!
//! The rows themselves — the join, their order, and what each column prints — are [`rows`]' job; this
//! module only decides where the sheet boundaries fall and writes them out. One partition's cells are
//! materialized at a time, so the peak is a sheet's worth of cells rather than the whole census.

use crate::report::{ReportError, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{write_sheet, Cell};

mod rows;

use rows::{performance_rows, PerformanceRow};

/// Rows one Excel worksheet holds.
const EXCEL_ROWS_PER_SHEET: usize = 1_048_576;

/// The column header, which every partition repeats above its own rows.
const HEADER_ROWS: usize = 1;

/// Rows held back from Excel's cap on purpose: the data budget stays a round number, and the last
/// row a partition writes sits [`PARTITION_MARGIN`] rows clear of the cap.
const PARTITION_MARGIN: usize = 48_575;

/// Data rows one `Performances_00N` sheet holds: `1_048_576 - 1 - 48_575`.
const DATA_ROWS_PER_SHEET: usize = EXCEL_ROWS_PER_SHEET - HEADER_ROWS - PARTITION_MARGIN;

/// The §52 columns in the objective's order, each with the width it is written at. The header row and
/// the column widths both come from here, so the two can never disagree.
const COLUMNS: [(&str, u16); 19] = [
    ("Athlete ID", 14),
    ("Athlete", 24),
    ("School", 28),
    ("Graduation Year", 15),
    ("Meet ID", 14),
    ("Meet", 34),
    ("Date", 12),
    ("State", 7),
    ("Sport", 14),
    ("Event", 16),
    ("Mark", 11),
    ("Normalized Mark", 16),
    ("Timing", 8),
    ("Wind", 7),
    ("Round", 11),
    ("Place", 7),
    ("Source", 18),
    ("Source ResultID", 22),
    ("Source URL", 44),
];

/// Write the §52 `Performances_00N` sheets: every performance `store` holds under `scope`.
pub(super) fn write_performance_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    scope: Scope,
) -> ReportResult<()> {
    let rows = performance_rows(store, scope)?;
    write_partitions(book, path, &rows, DATA_ROWS_PER_SHEET)
}

/// Write `rows` into `book`, `per_sheet` data rows to a sheet.
///
/// `per_sheet` is a parameter rather than the constant alone so the split is provable at test scale,
/// and so a budget that holds nothing is reachable and rejected instead of silently dropping rows.
fn write_partitions(
    book: &mut Workbook,
    path: &Path,
    rows: &[PerformanceRow],
    per_sheet: usize,
) -> ReportResult<()> {
    if per_sheet == 0 {
        return Err(no_budget(rows));
    }
    let widths: Vec<u16> = COLUMNS.iter().map(|(_, width)| *width).collect();
    let mut sheets = 0_usize;
    for partition in rows.chunks(per_sheet) {
        write_partition(book, path, sheets, partition, &widths)?;
        sheets = sheets.saturating_add(1);
    }
    if sheets == 0 {
        // A census with no performances in scope still publishes the sheet, header and all: an
        // absent sheet would read as "the run never got there".
        write_partition(book, path, 0, &[], &widths)?;
    }
    Ok(())
}

/// One `Performances_00N` sheet: the repeated header, then the partition's rows.
fn write_partition(
    book: &mut Workbook,
    path: &Path,
    index: usize,
    partition: &[PerformanceRow],
    widths: &[u16],
) -> ReportResult<()> {
    let mut rows: Vec<Vec<Cell>> = Vec::with_capacity(partition.len().saturating_add(HEADER_ROWS));
    rows.push(header());
    rows.extend(partition.iter().map(PerformanceRow::cells));
    write_sheet(book, path, &sheet_name(index), rows, widths, true)
}

/// The header row every partition repeats, in [`COLUMNS`] order.
fn header() -> Vec<Cell> {
    COLUMNS
        .iter()
        .map(|(label, _)| Cell::text(*label))
        .collect()
}

/// The name of the `index`-th partition, zero-padded as §52 publishes it. The padding is a minimum
/// width, so `Performances_1000` follows `Performances_999` instead of colliding with it.
fn sheet_name(index: usize) -> String {
    format!("Performances_{:03}", index.saturating_add(1))
}

/// The rejection for a budget that cannot hold a row: the first row that would have been written is
/// named, so nothing is dropped quietly.
fn no_budget(rows: &[PerformanceRow]) -> ReportError {
    let first = rows
        .first()
        .map(|row| row.id.as_str())
        .unwrap_or("(no performance rows)");
    ReportError::Invariant {
        detail: format!(
            "a sheet budget of 0 data rows cannot hold a performance; first row {first}"
        ),
    }
}

#[cfg(test)]
mod tests;
