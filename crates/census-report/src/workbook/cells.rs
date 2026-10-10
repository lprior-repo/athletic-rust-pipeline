use crate::report::{xlsx_error, ReportError, ReportResult};
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use std::path::Path;

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

    pub(super) fn number(value: usize) -> ReportResult<Self> {
        Ok(Cell::Number(count_as_number(value)?))
    }
}

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

pub(super) fn cell(value: impl Into<Cell>) -> Cell {
    value.into()
}

macro_rules! row {
    () => { Vec::new() };
    ($($value:expr),+ $(,)?) => { vec![$($crate::workbook::cells::cell($value)),+] };
}
pub(super) use row;

pub(super) struct SheetLayout<'a> {
    pub(super) name: &'a str,
    pub(super) widths: &'a [u16],
    pub(super) chrome: SheetChrome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SheetChrome {
    Filtered,
    Plain,
}

pub(super) fn write_sheet(
    book: &mut Workbook,
    path: &Path,
    layout: SheetLayout<'_>,
    rows: Vec<Vec<Cell>>,
) -> ReportResult<()> {
    let mut writer = SheetWriter::start(book, path, layout.name, layout.widths)?;
    let last_column = writer.write_rows(&rows)?;
    writer.finish(rows.len(), last_column, layout.chrome)
}

pub(super) struct SheetWriter<'a> {
    sheet: &'a mut Worksheet,
    path: &'a Path,
    bold: Format,
    next: usize,
}

impl<'a> SheetWriter<'a> {
    pub(super) fn start(
        book: &'a mut Workbook,
        path: &'a Path,
        name: &str,
        widths: &[u16],
    ) -> ReportResult<Self> {
        let mut writer = Self {
            sheet: book.add_worksheet_with_constant_memory(),
            path,
            bold: Format::new().set_bold(),
            next: 0,
        };
        writer.setup(name, widths)?;
        Ok(writer)
    }

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

    pub(super) fn write_row(&mut self, index: usize, cells: &[Cell]) -> ReportResult<()> {
        let row = self.check_row(index)?;
        for (column, cell) in cells.iter().enumerate() {
            let column = u16::try_from(column).map_err(|_| ReportError::Invariant {
                detail: "column index does not fit u16".to_string(),
            })?;
            self.write_cell(row, column, cell)?;
        }
        self.advance_row(index)
    }

    pub(super) fn write_strings(&mut self, index: usize, values: &[String]) -> ReportResult<()> {
        let row = self.check_row(index)?;
        for (column, value) in values.iter().enumerate() {
            let column = u16::try_from(column).map_err(|_| ReportError::Invariant {
                detail: "column index does not fit u16".to_string(),
            })?;
            self.write_text(row, column, value)?;
        }
        self.advance_row(index)
    }

    fn check_row(&self, index: usize) -> ReportResult<u32> {
        if index != self.next {
            return Err(ReportError::Invariant {
                detail: "constant-memory worksheets require sequential row writes".to_string(),
            });
        }
        u32::try_from(index).map_err(|_| ReportError::Invariant {
            detail: "row index does not fit u32".to_string(),
        })
    }

    fn advance_row(&mut self, index: usize) -> ReportResult<()> {
        if index != self.next {
            return Err(ReportError::Invariant {
                detail: "the worksheet row count advanced past its writes".to_string(),
            });
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the worksheet wrote more rows than it can count".to_string(),
            })?;
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        rows: usize,
        last_column: usize,
        chrome: SheetChrome,
    ) -> ReportResult<()> {
        if rows != self.next {
            return Err(ReportError::Invariant {
                detail: "the finished row count does not match the sequential writes".to_string(),
            });
        }
        if chrome == SheetChrome::Filtered && rows > 0 {
            self.autofilter(rows, last_column)?;
        }
        self.freeze_header()
    }

    fn write_rows(&mut self, rows: &[Vec<Cell>]) -> ReportResult<usize> {
        let last_column = rows
            .iter()
            .map(Vec::len)
            .max()
            .map_or(1, |value| value)
            .saturating_sub(1);
        for (index, cells) in rows.iter().enumerate() {
            self.write_row(index, cells)?;
        }
        Ok(last_column)
    }

    fn write_cell(&mut self, row: u32, column: u16, cell: &Cell) -> ReportResult<()> {
        match cell {
            Cell::Text(value) => self.write_text(row, column, value)?,
            Cell::Number(value) => {
                self.sheet
                    .write_number(row, column, *value)
                    .map_err(|source| xlsx_error(self.path, source))?;
            }
            Cell::Empty => {}
        }
        Ok(())
    }

    fn write_text(&mut self, row: u32, column: u16, value: &str) -> ReportResult<()> {
        if row == 0 {
            self.sheet
                .write_string_with_format(row, column, value, &self.bold)
                .map_err(|source| xlsx_error(self.path, source))?;
        } else {
            self.sheet
                .write_string(row, column, value)
                .map_err(|source| xlsx_error(self.path, source))?;
        }
        Ok(())
    }

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

    fn freeze_header(&mut self) -> ReportResult<()> {
        self.sheet
            .set_freeze_panes(1, 0)
            .map_err(|source| xlsx_error(self.path, source))?;
        Ok(())
    }
}
