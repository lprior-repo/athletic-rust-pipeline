use crate::report::{Derivation, ReportError, ReportResult};
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{Cell, SheetWriter};

mod join;
mod rows;
mod spill;

pub use join::PerformanceProjection;
pub use rows::{PerformanceRow, ProjectedValue};
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
    let layout = PartitionLayout::new();
    let mut rows = rows.into_iter();
    if per_sheet == 0 {
        let first = rows.next().transpose()?;
        return Err(no_budget(first.as_ref()));
    }
    let mut next = rows.next().transpose()?;
    let mut pending = PartitionRows {
        rows: &mut rows,
        next: &mut next,
        per_sheet,
    };
    layout.write_all(book, path, &mut pending)?;
    Ok(())
}

struct PartitionLayout {
    widths: Vec<u16>,
    header: Vec<Cell>,
    last_column: usize,
}

struct PartitionRows<'a, I> {
    rows: &'a mut I,
    next: &'a mut Option<PerformanceRow>,
    per_sheet: usize,
}

impl PartitionLayout {
    fn new() -> Self {
        Self {
            widths: COLUMNS.iter().map(|(_, width)| *width).collect(),
            header: header(),
            last_column: COLUMNS.len().saturating_sub(1),
        }
    }

    fn write_all<I: Iterator<Item = ReportResult<PerformanceRow>>>(
        &self,
        book: &mut Workbook,
        path: &Path,
        pending: &mut PartitionRows<'_, I>,
    ) -> ReportResult<()> {
        let mut sheets = 0_usize;
        while pending.next.is_some() {
            let filled = self.write(book, path, sheets, pending)?;
            sheets = sheets.saturating_add(1);
            if filled < pending.per_sheet {
                break;
            }
        }
        if sheets == 0 {
            self.write_empty(book, path)?;
        }
        Ok(())
    }

    fn write<I: Iterator<Item = ReportResult<PerformanceRow>>>(
        &self,
        book: &mut Workbook,
        path: &Path,
        index: usize,
        pending: &mut PartitionRows<'_, I>,
    ) -> ReportResult<usize> {
        let mut sheet = SheetWriter::start(book, path, &sheet_name(index), &self.widths)?;
        sheet.write_row(0, &self.header)?;
        let mut filled = 0_usize;
        while filled < pending.per_sheet {
            let Some(row) = pending.next.take() else {
                break;
            };
            sheet.write_row(filled.saturating_add(HEADER_ROWS), &row.cells())?;
            filled = filled.saturating_add(1);
            *pending.next = pending.rows.next().transpose()?;
        }
        sheet.finish(filled.saturating_add(HEADER_ROWS), self.last_column, true)?;
        Ok(filled)
    }

    fn write_empty(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        let mut sheet = SheetWriter::start(book, path, &sheet_name(0), &self.widths)?;
        sheet.write_row(0, &self.header)?;
        sheet.finish(HEADER_ROWS, self.last_column, true)
    }
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
        .map_or("(no performance rows)", |value| value);
    ReportError::Invariant {
        detail: format!(
            "a sheet budget of 0 data rows cannot hold a performance; first row {first}"
        ),
    }
}

#[cfg(test)]
mod tests;
