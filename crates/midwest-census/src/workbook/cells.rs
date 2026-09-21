//! The cell vocabulary: what a sheet is made of, and the one function that writes it out.
//!
//! A sheet is a `Vec<Vec<Cell>>`; `row!` builds a mixed row out of borrowed labels, owned strings,
//! numbers and explicit blanks; `write_sheet` is the only place that touches `rust_xlsxwriter`.

use anyhow::{Context, Result};
use rust_xlsxwriter::{Format, Workbook};

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
    pub(super) fn number(value: usize) -> Result<Self> {
        Ok(Cell::Number(count_as_number(value)?))
    }
}

/// Excel cells are `f64`, exact for every census count up to `u32::MAX`; larger counts are an error.
fn count_as_number(value: usize) -> Result<f64> {
    let value = u32::try_from(value).context("cell count does not fit u32")?;
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

pub(super) fn write_sheet(
    book: &mut Workbook,
    name: &str,
    rows: Vec<Vec<Cell>>,
    widths: &[u16],
    autofilter: bool,
) -> Result<()> {
    let bold = Format::new().set_bold();
    let sheet = book.add_worksheet();
    sheet.set_name(name)?;
    for (index, width) in widths.iter().enumerate() {
        let column = u16::try_from(index).context("column index does not fit u16")?;
        sheet.set_column_width(column, f64::from(*width))?;
    }
    let last_column = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .saturating_sub(1);
    for (row_index, cells) in rows.iter().enumerate() {
        let row_index = u32::try_from(row_index).context("row index does not fit u32")?;
        for (column, cell) in cells.iter().enumerate() {
            let column = u16::try_from(column).context("column index does not fit u16")?;
            match cell {
                Cell::Text(value) => {
                    if row_index == 0 {
                        sheet.write_string_with_format(row_index, column, value, &bold)?;
                    } else {
                        sheet.write_string(row_index, column, value)?;
                    }
                }
                Cell::Number(value) => {
                    sheet.write_number(row_index, column, *value)?;
                }
                Cell::Empty => {}
            }
        }
    }
    if autofilter && !rows.is_empty() {
        let last_row =
            u32::try_from(rows.len().saturating_sub(1)).context("row index does not fit u32")?;
        let last_column = u16::try_from(last_column).context("column index does not fit u16")?;
        sheet.autofilter(0, 0, last_row, last_column)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    Ok(())
}

pub(super) fn share(part: usize, whole: usize) -> Result<Cell> {
    if whole == 0 {
        return Ok(Cell::text("n/a"));
    }
    let part = count_as_number(part)?;
    let whole = count_as_number(whole)?;
    Ok(Cell::text(format!("{:.1}%", 100.0 * part / whole)))
}
