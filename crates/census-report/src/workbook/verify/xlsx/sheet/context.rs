use super::super::xml::{Element, Handler};
use super::{Cells, Walk};
use crate::report::ReportResult;

const WORKSHEET: u8 = 1;
const SHEET_DATA: u8 = 2;
const ROW: u8 = 3;
const CELL: u8 = 4;
const VALUE: u8 = 5;
const INLINE: u8 = 6;
const TEXT: u8 = 7;
const FORMULA: u8 = 8;
const OTHER: u8 = 9;
const RUN: u8 = 10;
const DIMENSION: u8 = 11;
const SPREADSHEET_NAMESPACE: u8 = 0x80;
const CODE_MASK: u8 = 0x7f;
const NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

impl<C: Cells> Walk<'_, C> {
    fn check_context(&self, code: u8, parent: Option<u8>) -> ReportResult<()> {
        let valid = match code {
            WORKSHEET => parent.is_none() && !self.root_seen,
            SHEET_DATA => parent == Some(WORKSHEET) && !self.sheet_data_seen,
            ROW => parent == Some(SHEET_DATA),
            CELL => parent == Some(ROW),
            VALUE | INLINE => parent == Some(CELL),
            RUN => parent == Some(INLINE),
            TEXT => matches!(parent, Some(INLINE | RUN)),
            DIMENSION => {
                parent == Some(WORKSHEET) && !self.sheet_data_seen && self.dimension.is_none()
            }
            _ => parent.is_some() && !matches!(parent, Some(VALUE | TEXT)),
        };
        if !valid {
            return Err(self
                .scope
                .corrupt("the sheet contains an element outside its immediate parent context"));
        }
        Ok(())
    }

    fn frame(&self, element: &Element<'_>) -> ReportResult<u8> {
        let parent = self.stack.last().copied();
        let inherited = parent.is_some_and(|frame| frame & SPREADSHEET_NAMESPACE != 0);
        let namespace = element
            .attribute("xmlns")
            .map_or(inherited, |uri| uri == NAMESPACE);
        let code = code(element.name);
        if element.qualified_name != element.name || (code != OTHER && !namespace) {
            return Err(self
                .scope
                .corrupt("the sheet contains an unsupported element name or namespace"));
        }
        if code == FORMULA {
            return Err(self
                .scope
                .outside("formula cells cannot be verified from cached values"));
        }
        self.check_context(code, parent.map(|frame| frame & CODE_MASK))?;
        Ok(code | if namespace { SPREADSHEET_NAMESPACE } else { 0 })
    }

    fn begin_value(&mut self, code: u8) -> ReportResult<()> {
        if (code == INLINE) != self.inline_string {
            return Err(self
                .scope
                .corrupt("a cell value container differs from its declared type"));
        }
        if self.value_seen {
            return Err(self
                .scope
                .corrupt("a cell contains more than one value container"));
        }
        self.value_seen = true;
        Ok(())
    }
}

impl<C: Cells> Handler for Walk<'_, C> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        let frame = self.frame(element)?;
        self.stack.push(frame);
        match frame & CODE_MASK {
            WORKSHEET => self.root_seen = true,
            SHEET_DATA => self.sheet_data_seen = true,
            CELL => self.begin_cell(element)?,
            code @ (VALUE | INLINE) => self.begin_value(code)?,
            DIMENSION => self.declare(element)?,
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, name: &str) -> ReportResult<()> {
        let code = code(name);
        if self.stack.pop().map(|frame| frame & CODE_MASK) != Some(code) {
            return Err(self
                .scope
                .corrupt(format!("sheet {} nests {name} inconsistently", self.sheet)));
        }
        if code == CELL {
            self.end_cell()?;
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> ReportResult<()> {
        let parent = self.stack.last().map(|frame| frame & CODE_MASK);
        if !matches!(parent, Some(VALUE | TEXT)) {
            return Ok(());
        }
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
        "r" => RUN,
        "dimension" => DIMENSION,
        _ => OTHER,
    }
}
