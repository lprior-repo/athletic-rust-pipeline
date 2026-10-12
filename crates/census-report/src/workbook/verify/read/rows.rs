use super::{visit_row, SparseRow};
use crate::report::ReportResult;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::xlsx;

pub(super) struct Rows<'a, F> {
    visit: &'a mut F,
    next: usize,
    held: Option<SparseRow>,
}

impl<'a, F: FnMut(&SparseRow)> Rows<'a, F> {
    pub(super) fn new(visit: &'a mut F) -> Self {
        Self {
            visit,
            next: 0,
            held: None,
        }
    }
}

impl<F: FnMut(&SparseRow)> xlsx::Cells for Rows<'_, F> {
    fn cell(&mut self, row: usize, column: usize, value: Value) -> ReportResult<()> {
        if self
            .held
            .as_ref()
            .is_none_or(|current| current.index != row)
        {
            if let Some(finished) = self.held.take() {
                self.next = visit_row(&finished, self.next, self.visit);
            }
            self.held = Some(SparseRow::new(row));
        }
        if let Some(current) = self.held.as_mut() {
            current.cells.insert(column, value);
        }
        Ok(())
    }

    fn finish(&mut self, rows: usize) {
        if let Some(finished) = self.held.take() {
            self.next = visit_row(&finished, self.next, self.visit);
        }
        for index in self.next..rows {
            (self.visit)(&SparseRow::new(index));
        }
    }
}
