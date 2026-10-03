use crate::report::ReportResult;
use crate::workbook::{PerformanceProjection, PerformanceRow, ProjectedValue};
use std::collections::BTreeMap;

use super::canonical::{cell_at, Value};
use super::expectations::{performance_sheet_names, Expectations, DATA_ROWS_PER_SHEET};
use super::labels;
use super::read::{
    compare, verify_header, verify_width, Book, Budget, SparseRow, EXCEL_COLUMNS_PER_SHEET,
};
use super::report::Findings;

type Key = (String, String, String, String, String, String);

struct ResultKey {
    key: Key,
    index: usize,
}

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    Reader::new(expectations, findings).run(book)
}

struct Reader<'a, 'd> {
    expectations: &'a Expectations<'d>,
    projection: PerformanceProjection<'a>,
    ordered: Vec<ResultKey>,
    actual: BTreeMap<String, usize>,
    seen: usize,
    findings: &'a mut Findings,
}

impl<'a, 'd> Reader<'a, 'd> {
    fn new(expectations: &'a Expectations<'d>, findings: &'a mut Findings) -> Self {
        let projection = PerformanceProjection::of(&expectations.derivation);
        let mut ordered: Vec<ResultKey> = expectations
            .derivation
            .performances()
            .iter()
            .enumerate()
            .map(|(index, performance)| ResultKey {
                key: key_of(&projection.row(performance)),
                index,
            })
            .collect();
        ordered.sort_by(|left, right| left.key.cmp(&right.key));
        Self {
            expectations,
            projection,
            ordered,
            actual: BTreeMap::new(),
            seen: 0_usize,
            findings,
        }
    }

    fn run(mut self, book: &mut Book) -> ReportResult<()> {
        for (index, name) in performance_sheet_names(self.ordered.len())
            .iter()
            .enumerate()
        {
            self.sheet(book, index, name)?;
        }
        self.report();
        Ok(())
    }

    fn sheet(&mut self, book: &mut Book, index: usize, name: &str) -> ReportResult<()> {
        let offset = index.saturating_mul(DATA_ROWS_PER_SHEET);
        let expected_here = self
            .ordered
            .len()
            .saturating_sub(offset)
            .min(DATA_ROWS_PER_SHEET);
        let mut seen = 0_usize;
        let mut visit = |row: &SparseRow| {
            verify_width(row, name, labels::PERFORMANCE_HEADERS.len(), self.findings);
            if row.index() == 0 {
                verify_header(row, name, &labels::PERFORMANCE_HEADERS, self.findings);
                return;
            }
            let position = offset.saturating_add(row.index().saturating_sub(1));
            if row.blank() {
                if let Some(entry) = self.ordered.get(position) {
                    self.findings.note(format!(
                        "{} is blank where result {} was expected",
                        cell_at(name, row.index(), 0),
                        self.id_at(entry.index)
                    ));
                }
                return;
            }
            match self.ordered.get(position).map(|entry| entry.index) {
                Some(index) => {
                    seen = seen.saturating_add(1);
                    if let Some(id) = row.text(0) {
                        let count = self.actual.entry(id.to_string()).or_insert(0);
                        *count = count.saturating_add(1);
                    }
                    self.compare(row, name, index);
                }
                None => self.findings.note(format!(
                    "{} carries an unexpected result row whose id reads {:?}",
                    cell_at(name, row.index(), 0),
                    row.text(0).map_or("", |value| value)
                )),
            }
        };
        let budget = Budget {
            rows: DATA_ROWS_PER_SHEET.saturating_add(1),
            columns: EXCEL_COLUMNS_PER_SHEET,
        };
        let Some(shape) = book.read(name, budget, &mut visit)? else {
            self.findings.note(format!(
                "sheet {name} is missing, so up to {expected_here} written result rows were never \
                 read"
            ));
            return Ok(());
        };
        let written = shape.rows.saturating_sub(1);
        self.report_sheet_counts(seen, written, name, offset, expected_here);
        Ok(())
    }

    fn report(&mut self) {
        self.findings.rows(self.seen);
        for (id, count) in &self.actual {
            if *count > 1 {
                self.findings.note(format!(
                    "result {id} appears {count} times across the performance sheets (duplicate rows)"
                ));
            }
        }
        if self.seen != self.ordered.len() {
            self.findings.note(format!(
                "the performance sheets carry {} result rows where the frozen dataset holds {}",
                self.seen,
                self.ordered.len()
            ));
        }
    }

    fn compare(&mut self, row: &SparseRow, name: &str, index: usize) {
        let Some(performance) = self.expectations.derivation.performances().get(index) else {
            self.findings.note(format!(
                "the frozen dataset has no performance at index {}",
                index
            ));
            return;
        };
        let projected = self.projection.row(performance);
        for (column, value) in projected.values().iter().enumerate() {
            compare(row, name, column, &Value::projected(*value), self.findings);
        }
    }

    fn id_at(&self, index: usize) -> String {
        self.expectations
            .derivation
            .performances()
            .get(index)
            .map(|performance| performance.id.as_str().to_string())
            .map_or(Default::default(), core::convert::identity)
    }

    fn report_sheet_counts(
        &mut self,
        seen: usize,
        written: usize,
        name: &str,
        offset: usize,
        expected_here: usize,
    ) {
        self.seen = self.seen.saturating_add(seen);
        if written < expected_here {
            let first = self
                .ordered
                .get(offset.saturating_add(written))
                .map(|entry| self.id_at(entry.index))
                .map_or(Default::default(), core::convert::identity);
            self.findings.note(format!(
                "sheet {name} holds {written} result rows where the frozen dataset requires \
                 {expected_here}; the first unread result is {first}"
            ));
        }
    }
}

fn key_of(row: &PerformanceRow) -> Key {
    let values = row.values();
    (
        text_at(&values, 3),
        text_at(&values, 7),
        text_at(&values, 2),
        text_at(&values, 1),
        text_at(&values, 10),
        text_at(&values, 0),
    )
}

fn text_at(values: &[ProjectedValue<'_>], index: usize) -> String {
    match values.get(index) {
        Some(ProjectedValue::Text(text)) => (*text).to_string(),
        _ => String::new(),
    }
}
