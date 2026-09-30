use crate::report::{Derivation, ReportError, ReportResult};
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{Cell, SheetWriter};

mod join;
mod rows;
mod spill;

use rows::PerformanceRow;
use spill::PerformanceRows;

const EXCEL_ROWS_PER_SHEET: usize = 1_048_576;

const HEADER_ROWS: usize = 1;

const PARTITION_MARGIN: usize = 48_575;

const DATA_ROWS_PER_SHEET: usize = EXCEL_ROWS_PER_SHEET - HEADER_ROWS - PARTITION_MARGIN;

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

pub(super) fn write_performance_sheets(
    book: &mut Workbook,
    path: &Path,
    derivation: &Derivation<'_>,
) -> ReportResult<()> {
    let rows = PerformanceRows::build(derivation)?;
    write_partitions(book, path, rows, DATA_ROWS_PER_SHEET)
}

fn write_partitions(
    book: &mut Workbook,
    path: &Path,
    rows: impl IntoIterator<Item = ReportResult<PerformanceRow>>,
    per_sheet: usize,
) -> ReportResult<()> {
    let widths: Vec<u16> = COLUMNS.iter().map(|(_, width)| *width).collect();
    let header_row = header();
    let last_column = COLUMNS.len().saturating_sub(1);
    let mut rows = rows.into_iter();
    if per_sheet == 0 {
        let first = rows.next().transpose()?;
        return Err(no_budget(first.as_ref()));
    }
    let mut sheets = 0_usize;
    let mut next = rows.next().transpose()?;
    while next.is_some() {
        let filled = {
            let mut sheet = SheetWriter::start(book, path, &sheet_name(sheets), &widths)?;
            sheet.write_row(0, &header_row)?;
            let mut filled = 0_usize;
            while filled < per_sheet {
                let Some(row) = next.take() else { break };
                sheet.write_row(filled.saturating_add(HEADER_ROWS), &row.cells())?;
                filled = filled.saturating_add(1);
                next = rows.next().transpose()?;
            }
            sheet.finish(filled.saturating_add(HEADER_ROWS), last_column, true)?;
            filled
        };
        sheets = sheets.saturating_add(1);
        if filled < per_sheet {
            break;
        }
    }
    if sheets == 0 {
        let mut sheet = SheetWriter::start(book, path, &sheet_name(0), &widths)?;
        sheet.write_row(0, &header_row)?;
        sheet.finish(HEADER_ROWS, last_column, true)?;
    }
    Ok(())
}

fn header() -> Vec<Cell> {
    COLUMNS
        .iter()
        .map(|(label, _)| Cell::text(*label))
        .collect()
}

fn sheet_name(index: usize) -> String {
    format!("Performances_{:03}", index.saturating_add(1))
}

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
