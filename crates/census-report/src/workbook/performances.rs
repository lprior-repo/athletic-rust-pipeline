//! The `Performances_00N` sheets: every performance the census holds, one row per mark.
//!
//! The objective (§52) asks the recruiting workbook for *every available performance*, which is more
//! rows than one Excel worksheet holds: a sheet is capped at 1,048,576 rows. The rows are therefore
//! partitioned into `Performances_001`, `Performances_002`, … — every sheet repeats the column header
//! and carries at most [`DATA_ROWS_PER_SHEET`] data rows, which is Excel's cap minus the header minus
//! a [`PARTITION_MARGIN`]-row margin. A row is never truncated: a budget that cannot hold one is a
//! rejection that names it, not a quietly shorter sheet.
//!
//! The rows themselves — the join, their order, and what each column prints — are [`join`]'s and
//! [`rows`]' job, and handing them over sorted without holding the whole table is [`spill`]'s; this
//! module only decides where the sheet boundaries fall and writes them out. The rows arrive through
//! [`PerformanceRows`], which spills them to disk and hands them over one school-name range at a
//! time, and each row goes into its sheet as it arrives, so the peak is a range's sorted rows plus the
//! row in flight rather than the whole census.

use crate::report::{ReportError, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{Cell, SheetWriter};

mod join;
mod rows;
mod spill;

use rows::PerformanceRow;
use spill::PerformanceRows;

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
const COLUMNS: [(&str, u16); 20] = [
    ("Canonical Result ID", 22),
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

/// The population declaration for the performance sheets.
///
/// Carries the cohort scope, athlete count, performance row count, and the rule that earlier-season
/// and out-of-state performances for cohort athletes are included by design — so the row count
/// reconciles with the seal.
#[derive(Debug, Clone, Copy)]
pub(super) struct PerformanceSheetPopulation {
    /// The graduating class the sheet covers, or `None` for "all cohorts".
    pub(super) cohort_year: Option<i16>,
    /// Evidence scope: `core` or `all_sources`.
    pub(super) scope: Scope,
    /// Cohort athletes the sheet draws from.
    pub(super) cohort_athletes: usize,
    /// Total performance rows written across all partitions.
    pub(super) total_rows: usize,
}

/// Write the §52 `Performances_00N` sheets and return a population declaration.
///
/// `grad_year` restricts the sheet to the cohort: only performances whose athlete falls within
/// the graduating-class filter appear, while every captured season for those athletes stays intact.
pub(super) fn write_performance_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    scope: Scope,
    grad_year: Option<i16>,
) -> ReportResult<PerformanceSheetPopulation> {
    let rows = PerformanceRows::build(store, scope, grad_year)?;
    let row_count = write_partitions_with_count(book, path, rows, DATA_ROWS_PER_SHEET)?;
    Ok(PerformanceSheetPopulation {
        cohort_year: grad_year,
        scope,
        cohort_athletes: 0, // Filled in by caller from recruiting dataset
        total_rows: row_count,
    })
}

/// Write `rows` into `book`, `per_sheet` data rows to a sheet, one row at a time: no sheet's cells and
/// no partition of the rows are ever held whole.
///
/// `per_sheet` is a parameter rather than the constant alone so the split is provable at test scale,
/// and so a budget that holds nothing is reachable and rejected instead of silently dropping rows.
///
/// Only the partition tests call this counted-eliding form; production goes through
/// [`write_partitions_with_count`] and declares the population it wrote.
#[cfg(test)]
fn write_partitions(
    book: &mut Workbook,
    path: &Path,
    rows: impl IntoIterator<Item = ReportResult<PerformanceRow>>,
    per_sheet: usize,
) -> ReportResult<()> {
    write_partitions_with_count(book, path, rows, per_sheet).map(|_| ())
}

/// Same as [`write_partitions`], but returns the total row count for population declaration.
fn write_partitions_with_count(
    book: &mut Workbook,
    path: &Path,
    rows: impl IntoIterator<Item = ReportResult<PerformanceRow>>,
    per_sheet: usize,
) -> ReportResult<usize> {
    let widths: Vec<u16> = COLUMNS.iter().map(|(_, width)| *width).collect();
    let header_row = header();
    let last_column = COLUMNS.len().saturating_sub(1);
    let mut rows = rows.into_iter();
    if per_sheet == 0 {
        // The first row is pulled only to name it in the refusal: a budget of 0 drops rows, and the
        // operator is told which row it would have dropped.
        let first = rows.next().transpose()?;
        return Err(no_budget(first.as_ref()));
    }
    let mut sheets = 0_usize;
    let mut total_rows = 0_usize;
    // One row of lookahead, so a full last sheet is not followed by an empty one.
    let mut next = rows.next().transpose()?;
    while next.is_some() {
        let filled = {
            // The sheet's borrow of `book` ends with this block, so the next sheet can be added.
            let mut sheet = SheetWriter::start(book, path, &sheet_name(sheets), &widths)?;
            sheet.write_row(0, &header_row)?;
            let mut filled = 0_usize;
            while filled < per_sheet {
                let Some(row) = next.take() else { break };
                sheet.write_row(filled.saturating_add(HEADER_ROWS), &row.cells())?;
                filled = filled.saturating_add(1);
                next = rows.next().transpose()?;
            }
            sheet.finish(filled.saturating_add(HEADER_ROWS), last_column)?;
            filled
        };
        total_rows = total_rows.saturating_add(filled);
        sheets = sheets.saturating_add(1);
        if filled < per_sheet {
            break;
        }
    }
    if sheets == 0 {
        // A census with no performances in scope still publishes the sheet, header and all: an
        // absent sheet would read as "the run never got there".
        let mut sheet = SheetWriter::start(book, path, &sheet_name(0), &widths)?;
        sheet.write_row(0, &header_row)?;
        sheet.finish(HEADER_ROWS, last_column)?;
    }
    Ok(total_rows)
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
fn no_budget(row: Option<&PerformanceRow>) -> ReportError {
    let first = row
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
