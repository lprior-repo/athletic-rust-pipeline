use crate::report::{io_error, ReportError, ReportResult};
use calamine::{Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use super::canonical::{cell_at, Value};
use super::expectations::Expectations;
use super::report::Findings;

pub(super) const EXCEL_ROWS_PER_SHEET: usize = 1_048_576;

pub(super) const EXCEL_COLUMNS_PER_SHEET: usize = 16_384;

#[derive(Debug, Clone, Copy)]
pub(super) struct Budget {
    pub(super) rows: usize,
    pub(super) columns: usize,
}

impl Budget {
    fn validate(self, name: &str, rows: usize, columns: usize) -> ReportResult<()> {
        if rows > self.rows || columns > self.columns {
            return Err(ReportError::Invariant {
                detail: format!(
                    "sheet {name} declares {rows} rows by {columns} columns, beyond the {} row by \
                     {} column readback budget; the frozen readback reads every cell and never \
                     samples or strides",
                    self.rows, self.columns
                ),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Shape {
    pub(super) rows: usize,
}

pub(super) struct Book {
    path: PathBuf,
    workbook: Xlsx<BufReader<File>>,
}

impl Book {
    pub(super) fn open(path: &Path) -> ReportResult<Self> {
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        let workbook =
            Xlsx::new(BufReader::new(file)).map_err(|source| unreadable(path, source))?;
        Ok(Self {
            path: path.to_path_buf(),
            workbook,
        })
    }

    pub(super) fn names(&self) -> Vec<String> {
        self.workbook.sheet_names().to_vec()
    }

    pub(super) fn read(
        &mut self,
        name: &str,
        budget: Budget,
        mut visit: impl FnMut(&SparseRow),
    ) -> ReportResult<Option<Shape>> {
        if !self.names().iter().any(|sheet| sheet == name) {
            return Ok(None);
        }
        let mut reader = self
            .workbook
            .worksheet_cells_reader(name)
            .map_err(|source| unreadable(&self.path, source))?;
        let dimensions = reader.dimensions();
        let rows = usize::try_from(dimensions.end.0)
            .map_or(usize::MAX, |value| value)
            .saturating_add(1);
        let columns = usize::try_from(dimensions.end.1)
            .map_or(usize::MAX, |value| value)
            .saturating_add(1);
        budget.validate(name, rows, columns)?;
        let mut held: Option<SparseRow> = None;
        let mut next = 0_usize;
        while let Some(cell) = reader
            .next_cell()
            .map_err(|source| unreadable(&self.path, source))?
        {
            let row = usize::try_from(cell.get_position().0).map_or(usize::MAX, |value| value);
            let column = usize::try_from(cell.get_position().1).map_or(usize::MAX, |value| value);
            if row >= budget.rows || column >= budget.columns {
                return Err(ReportError::Invariant {
                    detail: format!(
                        "sheet {name} contains an out-of-budget cell at {row},{column}"
                    ),
                });
            }
            if held.as_ref().is_none_or(|current| current.index != row) {
                if let Some(finished) = held.take() {
                    next = visit_row(&finished, next, &mut visit);
                }
                held = Some(SparseRow::new(row));
            }
            if let Some(current) = held.as_mut() {
                current
                    .cells
                    .insert(column, Value::from_ref(cell.get_value()));
            }
        }
        if let Some(finished) = held.take() {
            visit_row(&finished, next, &mut visit);
        }
        Ok(Some(Shape { rows }))
    }
}

fn visit_row(row: &SparseRow, next: usize, visit: &mut impl FnMut(&SparseRow)) -> usize {
    let mut index = next;
    while index < row.index {
        visit(&SparseRow::new(index));
        index = index.saturating_add(1);
    }
    visit(row);
    row.index.saturating_add(1)
}

pub(super) struct SparseRow {
    index: usize,
    cells: BTreeMap<usize, Value>,
}

impl SparseRow {
    fn new(index: usize) -> Self {
        Self {
            index,
            cells: BTreeMap::new(),
        }
    }

    pub(super) fn index(&self) -> usize {
        self.index
    }

    pub(super) fn blank(&self) -> bool {
        self.cells.values().all(Value::is_empty)
    }

    pub(super) fn get(&self, column: usize) -> Option<&Value> {
        self.cells.get(&column).filter(|value| !value.is_empty())
    }

    pub(super) fn text(&self, column: usize) -> Option<&str> {
        match self.cells.get(&column) {
            Some(Value::Text(text)) => Some(text.as_str()),
            _ => None,
        }
    }

    pub(super) fn extras(&self, columns: usize) -> impl Iterator<Item = (usize, &Value)> {
        self.cells
            .iter()
            .filter(move |(column, value)| **column >= columns && !value.is_empty())
            .map(|(column, value)| (*column, value))
    }
}

pub(super) fn compare(
    row: &SparseRow,
    sheet: &str,
    column: usize,
    expected: &Value,
    findings: &mut Findings,
) {
    let found = row.get(column);
    if found.map_or(&Value::Empty, |value| value) != expected {
        findings.mismatch(
            &cell_at(sheet, row.index(), column),
            expected,
            found.map_or(&Value::Empty, |value| value),
        );
    }
}

pub(super) fn verify_header(
    row: &SparseRow,
    sheet: &str,
    headers: &[&str],
    findings: &mut Findings,
) {
    for (column, header) in headers.iter().enumerate() {
        compare(row, sheet, column, &Value::text(*header), findings);
    }
}

pub(super) fn verify_width(row: &SparseRow, sheet: &str, columns: usize, findings: &mut Findings) {
    for (column, value) in row.extras(columns) {
        findings.note(format!(
            "{} holds a value outside the {columns} objective columns: {value}",
            cell_at(sheet, row.index(), column)
        ));
    }
}

pub(super) fn verify_inventory(
    book: &Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) {
    let names: BTreeSet<String> = book.names().into_iter().collect();
    let expected = expectations.sheet_names();
    for name in &expected {
        if !names.contains(name) {
            findings.note(format!("sheet {name} is missing from the workbook"));
        }
    }
    let known: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    for name in &names {
        if !known.contains(name.as_str()) {
            findings.note(format!(
                "sheet {name} is not part of the frozen publication objective"
            ));
        }
    }
}

fn unreadable(path: &Path, source: calamine::XlsxError) -> ReportError {
    ReportError::Invariant {
        detail: format!(
            "reading the frozen workbook {} with a strict reader: {source}",
            path.display()
        ),
    }
}
