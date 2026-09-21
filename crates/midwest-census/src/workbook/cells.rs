//! The cell vocabulary: what a sheet is made of, and the one function that writes it out.
//!
//! A sheet is a `Vec<Vec<Cell>>`; `row!` builds a mixed row out of borrowed labels, owned strings,
//! numbers and explicit blanks; `write_sheet` is the only place that touches `rust_xlsxwriter`.

use crate::report::{xlsx_error, ReportError, ReportResult};
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use std::path::Path;

/// One cell of a sheet.
#[derive(Debug, Clone)]
pub(super) enum Cell {
    Text(String),
    Number(f64),
    Empty,
}

impl Cell {
    pub(super) fn text(value: impl Into<String>) -> Self {
        Cell::Text(value.into())
    }

    /// A census count as the number an Excel cell holds.
    pub(super) fn number(value: usize) -> ReportResult<Self> {
        Ok(Cell::Number(count_as_number(value)?))
    }
}

/// Excel cells are `f64`, exact for every census count up to `u32::MAX`; larger counts are an error.
fn count_as_number(value: usize) -> ReportResult<f64> {
    let value = u32::try_from(value).map_err(|_| ReportError::Invariant {
        detail: "cell count does not fit u32".to_string(),
    })?;
    Ok(f64::from(value))
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell::Text(value.to_string())
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Cell::Text(value)
    }
}

/// Convert any supported cell source into a [`Cell`].
pub(super) fn cell(value: impl Into<Cell>) -> Cell {
    value.into()
}

/// Build one sheet row from heterogeneous values: `row!(Cell::text(label), count, Cell::Empty)`.
///
/// A plain function cannot do this: an array literal forces one element type, and real rows mix
/// borrowed labels, owned strings, numbers, and explicit blanks.
macro_rules! row {
    () => { Vec::new() };
    ($($value:expr),+ $(,)?) => { vec![$(cell($value)),+] };
}
pub(super) use row;

/// Write one sheet into `book`, naming `path` (the workbook being assembled) in every rejection, so
/// a refused sheet name or cell still points at the file the operator asked for.
pub(super) fn write_sheet(
    book: &mut Workbook,
    path: &Path,
    name: &str,
    rows: Vec<Vec<Cell>>,
    widths: &[u16],
    autofilter: bool,
) -> ReportResult<()> {
    let mut writer = SheetWriter {
        sheet: book.add_worksheet(),
        path,
        bold: Format::new().set_bold(),
    };
    writer.setup(name, widths)?;
    let last_column = writer.write_rows(&rows)?;
    if autofilter && !rows.is_empty() {
        writer.autofilter(rows.len(), last_column)?;
    }
    writer.freeze_header()
}

/// One worksheet being written, plus the workbook path every rejection names and the format the
/// header row uses.
struct SheetWriter<'a> {
    sheet: &'a mut Worksheet,
    path: &'a Path,
    bold: Format,
}

impl SheetWriter<'_> {
    /// Name the sheet and size its columns.
    fn setup(&mut self, name: &str, widths: &[u16]) -> ReportResult<()> {
        self.sheet
            .set_name(name)
            .map_err(|source| xlsx_error(self.path, source))?;
        for (index, width) in widths.iter().enumerate() {
            let column = u16::try_from(index).map_err(|_| ReportError::Invariant {
                detail: "column index does not fit u16".to_string(),
            })?;
            self.sheet
                .set_column_width(column, f64::from(*width))
                .map_err(|source| xlsx_error(self.path, source))?;
        }
        Ok(())
    }

    /// Write every cell, the header row bold; returns the widest row's last column index.
    fn write_rows(&mut self, rows: &[Vec<Cell>]) -> ReportResult<usize> {
        let last_column = rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .saturating_sub(1);
        for (row_index, cells) in rows.iter().enumerate() {
            let row_index = u32::try_from(row_index).map_err(|_| ReportError::Invariant {
                detail: "row index does not fit u32".to_string(),
            })?;
            for (column, cell) in cells.iter().enumerate() {
                let column = u16::try_from(column).map_err(|_| ReportError::Invariant {
                    detail: "column index does not fit u16".to_string(),
                })?;
                self.write_cell(row_index, column, cell)?;
            }
        }
        Ok(last_column)
    }

    /// Write one cell: text, a number, or nothing at all for a blank.
    fn write_cell(&mut self, row: u32, column: u16, cell: &Cell) -> ReportResult<()> {
        match cell {
            Cell::Text(value) => {
                if row == 0 {
                    self.sheet
                        .write_string_with_format(row, column, value, &self.bold)
                        .map_err(|source| xlsx_error(self.path, source))?;
                } else {
                    self.sheet
                        .write_string(row, column, value)
                        .map_err(|source| xlsx_error(self.path, source))?;
                }
            }
            Cell::Number(value) => {
                self.sheet
                    .write_number(row, column, *value)
                    .map_err(|source| xlsx_error(self.path, source))?;
            }
            Cell::Empty => {}
        }
        Ok(())
    }

    /// Extend the header row's autofilter over every written row and column.
    fn autofilter(&mut self, rows: usize, last_column: usize) -> ReportResult<()> {
        let last_row =
            u32::try_from(rows.saturating_sub(1)).map_err(|_| ReportError::Invariant {
                detail: "row index does not fit u32".to_string(),
            })?;
        let last_column = u16::try_from(last_column).map_err(|_| ReportError::Invariant {
            detail: "column index does not fit u16".to_string(),
        })?;
        self.sheet
            .autofilter(0, 0, last_row, last_column)
            .map_err(|source| xlsx_error(self.path, source))?;
        Ok(())
    }

    /// Keep the header row visible while the sheet scrolls.
    fn freeze_header(&mut self) -> ReportResult<()> {
        self.sheet
            .set_freeze_panes(1, 0)
            .map_err(|source| xlsx_error(self.path, source))?;
        Ok(())
    }
}

pub(super) fn share(part: usize, whole: usize) -> ReportResult<Cell> {
    if whole == 0 {
        return Ok(Cell::text("n/a"));
    }
    let part = count_as_number(part)?;
    let whole = count_as_number(whole)?;
    Ok(Cell::text(format!("{:.1}%", 100.0 * part / whole)))
}
