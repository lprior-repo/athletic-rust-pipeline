use super::xml::{Element, Handler};
use super::{Profile, Scope};
use crate::report::ReportResult;
use crate::workbook::verify::canonical::{cell_reference, Value};

mod reference;

use reference::{address, extent};

const WORKSHEET: u8 = 1;
const SHEET_DATA: u8 = 2;
const ROW: u8 = 3;
const CELL: u8 = 4;
const VALUE: u8 = 5;
const INLINE: u8 = 6;
const TEXT: u8 = 7;
const FORMULA: u8 = 8;
const OTHER: u8 = 9;

pub(in crate::workbook::verify) trait Cells {
    fn cell(&mut self, row: usize, column: usize, value: Value) -> ReportResult<()>;
    fn finish(&mut self, rows: usize);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    None,
    Value,
    Text,
    Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number,
    Text,
    Boolean,
}

pub(super) struct Walk<'a, C> {
    visit: &'a mut C,
    scope: Scope<'a>,
    profile: Profile,
    sheet: String,
    rows: usize,
    columns: usize,
    stack: Vec<u8>,
    dimension: Option<(usize, usize)>,
    position: Option<(usize, usize)>,
    kind: Kind,
    target: Target,
    value: String,
    seen: u64,
    extent: usize,
    previous: Option<(usize, usize)>,
}

impl<'a, C: Cells> Walk<'a, C> {
    pub(super) fn new(
        visit: &'a mut C,
        scope: Scope<'a>,
        profile: Profile,
        sheet: &str,
        rows: usize,
        columns: usize,
    ) -> Self {
        Self {
            visit,
            scope,
            profile,
            sheet: sheet.to_string(),
            rows,
            columns,
            stack: Vec::new(),
            dimension: None,
            position: None,
            kind: Kind::Number,
            target: Target::None,
            value: String::new(),
            seen: 0,
            extent: 0,
            previous: None,
        }
    }

    pub(super) fn finish(self) -> ReportResult<usize> {
        if !self.stack.is_empty() {
            return Err(self
                .scope
                .corrupt("the sheet XML ends with elements still open"));
        }
        let rows = self.dimension.map_or(self.extent, |dimension| dimension.0);
        self.visit.finish(rows);
        Ok(rows)
    }

    fn declare(&mut self, element: &Element<'_>) -> ReportResult<()> {
        let Some(reference) = element.attribute("ref") else {
            return Ok(());
        };
        let (rows, columns) = extent(reference).ok_or_else(|| {
            self.scope.corrupt(format!(
                "the sheet declares a malformed dimension {reference}"
            ))
        })?;
        if rows > self.rows || columns > self.columns {
            return Err(self.scope.corrupt(format!(
                "sheet {} declares {rows} rows by {columns} columns, beyond the {} row by {} \
                 column readback budget; the frozen readback reads every cell and never samples \
                 or strides",
                self.sheet, self.rows, self.columns
            )));
        }
        self.dimension = Some((rows, columns));
        Ok(())
    }

    fn begin_cell(&mut self, element: &Element<'_>) -> ReportResult<()> {
        let reference = element.required("r", &self.scope)?;
        let position = address(reference).ok_or_else(|| {
            self.scope
                .corrupt(format!("a cell reference {reference} is malformed"))
        })?;
        self.position = Some(position);
        self.kind = match element.attribute("t") {
            None | Some("n") => Kind::Number,
            Some("inlineStr" | "str" | "d" | "e") => Kind::Text,
            Some("b") => Kind::Boolean,
            Some("s") => {
                return Err(self.scope.outside(
                    "the workbook stores cell text in xl/sharedStrings.xml and this cell refers \
                     to it; the qualified readback reads inline strings only, so the sheet is \
                     refused rather than read with silently wrong values",
                ));
            }
            Some(other) => {
                return Err(self
                    .scope
                    .outside(format!("cell type {other} is not part of the profile")));
            }
        };
        self.value.clear();
        self.target = Target::None;
        Ok(())
    }

    fn end_cell(&mut self) -> ReportResult<()> {
        let Some((row, column)) = self.position.take() else {
            return Err(self
                .scope
                .corrupt("the sheet closes a cell that never opened"));
        };
        if row >= self.rows || column >= self.columns {
            return Err(self.scope.corrupt(format!(
                "sheet {} holds a cell at {row},{column} beyond its readback envelope",
                self.sheet
            )));
        }
        if let Some((rows, columns)) = self.dimension {
            if row >= rows || column >= columns {
                return Err(self.scope.corrupt(format!(
                    "sheet {} holds cell {} beyond its declared dimension",
                    self.sheet,
                    cell_reference(row, column)
                )));
            }
        }
        if let Some(previous) = self.previous {
            if (row, column) <= previous {
                return Err(self.scope.corrupt(format!(
                    "sheet {} revisits cell {} out of order",
                    self.sheet,
                    cell_reference(row, column)
                )));
            }
        }
        self.previous = Some((row, column));
        self.seen = self.seen.checked_add(1).ok_or_else(|| {
            self.scope
                .corrupt("the sheet cell count does not fit its counter")
        })?;
        if self.seen > self.profile.cells {
            return Err(self.scope.corrupt(format!(
                "sheet {} holds more than {} cells",
                self.sheet, self.profile.cells
            )));
        }
        self.extent = self.extent.max(row.saturating_add(1));
        let value = match self.kind {
            Kind::Number => self.number()?,
            Kind::Boolean => self.boolean()?,
            Kind::Text => Value::text(self.value.as_str()),
        };
        self.visit.cell(row, column, value)
    }

    fn number(&self) -> ReportResult<Value> {
        if self.value.is_empty() {
            return Ok(Value::Empty);
        }
        self.value
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| {
                self.scope.corrupt(format!(
                    "sheet {} holds a number cell that is not a number",
                    self.sheet
                ))
            })
    }

    fn boolean(&self) -> ReportResult<Value> {
        match self.value.trim() {
            "1" | "true" | "TRUE" => Ok(Value::text("true")),
            "0" | "false" | "FALSE" => Ok(Value::text("false")),
            _ => Err(self.scope.corrupt(format!(
                "sheet {} holds a boolean cell that is not a boolean",
                self.sheet
            ))),
        }
    }
}

impl<C: Cells> Handler for Walk<'_, C> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        let code = code(element.name);
        self.stack.push(code);
        match code {
            CELL => self.begin_cell(element)?,
            VALUE => self.target = Target::Value,
            TEXT => self.target = Target::Text,
            FORMULA => self.target = Target::Formula,
            _ => {}
        }
        if element.name == "dimension" {
            self.declare(element)?;
        }
        Ok(())
    }

    fn end(&mut self, name: &str) -> ReportResult<()> {
        let code = code(name);
        if self.stack.pop() != Some(code) {
            return Err(self
                .scope
                .corrupt(format!("sheet {} nests {name} inconsistently", self.sheet)));
        }
        match code {
            CELL => self.end_cell()?,
            VALUE | TEXT | FORMULA => self.target = Target::None,
            _ => {}
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> ReportResult<()> {
        match self.target {
            Target::None | Target::Formula => Ok(()),
            Target::Value | Target::Text => {
                if self.value.len().saturating_add(text.len()) > self.profile.token {
                    return Err(self.scope.corrupt(format!(
                        "sheet {} holds a cell value beyond the {} byte token budget",
                        self.sheet, self.profile.token
                    )));
                }
                self.value.push_str(text);
                Ok(())
            }
        }
    }
}

fn code(name: &str) -> u8 {
    match name {
        "worksheet" => WORKSHEET,
        "sheetData" => SHEET_DATA,
        "row" => ROW,
        "c" => CELL,
        "v" => VALUE,
        "is" => INLINE,
        "t" => TEXT,
        "f" => FORMULA,
        _ => OTHER,
    }
}
