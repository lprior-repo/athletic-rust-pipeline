use crate::export::ExportDataset;
use crate::report::{Derivation, ReportError, ReportResult, Scope};
use crate::workbook::meta::queues::{cohort_of, conflict_families, review_families};
use crate::workbook::meta::{school_name_index, sorted_counts, Family, StoreRows};
use crate::workbook::Censuses;
use census_domain::model::SchoolYear;
use std::path::{Path, PathBuf};

use super::canonical::{cell_at, Value};
use super::expectations::Expectations;
use super::read::{
    compare, verify_width, Book, Budget, Shape, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;

mod coverage;
mod meets;
mod metrics;
mod queues;
mod schools;
mod sources;

pub(super) enum Expect {
    Text(String),
    Number(f64),
    Empty,
}

impl Expect {
    pub(super) fn text(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(value.to_string())
        }
    }

    pub(super) fn count(value: usize) -> ReportResult<Self> {
        let value = u32::try_from(value).map_err(|_| ReportError::Invariant {
            detail: "a frozen count does not fit the u32 cell domain".to_string(),
        })?;
        Ok(Self::Number(f64::from(value)))
    }
}

pub(super) type Sheet = (&'static str, Vec<Vec<Expect>>);

pub(super) fn header(headers: &[&str]) -> Vec<Expect> {
    headers.iter().map(|header| Expect::text(*header)).collect()
}

pub(super) fn text_row(label: &str, value: &str) -> Vec<Expect> {
    vec![Expect::text(label), Expect::text(value)]
}

pub(super) fn number_row(label: &str, value: f64) -> Vec<Expect> {
    vec![Expect::text(label), Expect::Number(value)]
}

pub(super) struct Series<'a, 'd> {
    dataset: &'a ExportDataset,
    scope: Scope,
    school_year: SchoolYear,
    recruiting: &'a Derivation<'d>,
    bests: usize,
    population: &'a Derivation<'d>,
    censuses: &'a Censuses,
    rows: &'a StoreRows<'d>,
    index: &'a queues::Lookup<'d>,
}

impl Series<'_, '_> {
    fn sheets(&self) -> ReportResult<Vec<Sheet>> {
        let cohort = cohort_of(self.dataset, self.scope);
        let names = school_name_index(self.population.schools());
        let conflicts = conflict_families(self.rows, &cohort, &names);
        let review = review_families(self.rows, &cohort, &names)?;
        Ok(vec![
            schools::expected(self.population.schools())?,
            meets::expected(self.population.meets())?,
            sources::expected(self.censuses)?,
            coverage::expected(self.dataset)?,
            queues::conflicts(&conflicts, self.index),
            queues::review(&review, &self.dataset.verdicts, self.index),
            self.metrics(&conflicts)?,
        ])
    }

    fn metrics(&self, conflicts: &[Family]) -> ReportResult<Sheet> {
        metrics::expected(self, conflicts)
    }
}

fn expected_sheets(expectations: &Expectations<'_>) -> ReportResult<Vec<Sheet>> {
    let dataset = expectations.dataset;
    let scope = expectations.derivation.scope();
    let population = Derivation::of(dataset, scope, None);
    let censuses = Censuses::of(dataset, &census_out(dataset));
    let rows = StoreRows::of(&population, expectations.school_year)?;
    let index = queues::Lookup::of(
        population.schools(),
        population.meets(),
        population.athletes(),
    );
    let series = Series {
        dataset,
        scope,
        school_year: expectations.school_year,
        recruiting: &expectations.derivation,
        bests: expectations.bests.len(),
        population: &population,
        censuses: &censuses,
        rows: &rows,
        index: &index,
    };
    series.sheets()
}

fn census_out(dataset: &ExportDataset) -> PathBuf {
    Path::new(dataset.lineage.store_root.as_str()).join("out")
}

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let mut seen = 0_usize;
    for (name, rows) in expected_sheets(expectations)? {
        seen = seen.saturating_add(readback(book, name, &rows, findings)?);
    }
    findings.rows(seen);
    Ok(())
}

fn readback(
    book: &mut Book,
    sheet: &str,
    expected: &[Vec<Expect>],
    findings: &mut Findings,
) -> ReportResult<usize> {
    let mut seen = 0_usize;
    let mut visit = |row: &SparseRow| {
        let Some(cells) = expected.get(row.index()) else {
            if !row.blank() {
                findings.note(beyond(sheet, row.index(), expected.len()));
            }
            return;
        };
        seen = seen.saturating_add(1);
        compare_row(row, sheet, cells, findings);
    };
    match book.read(sheet, sheet_budget(), &mut visit)? {
        Some(shape) => shape_note(shape, sheet, expected.len(), findings),
        None => findings.note(unwritten(sheet, expected.len())),
    }
    Ok(seen)
}

fn sheet_budget() -> Budget {
    Budget {
        rows: EXCEL_ROWS_PER_SHEET,
        columns: EXCEL_COLUMNS_PER_SHEET,
    }
}

fn unwritten(sheet: &str, expected: usize) -> String {
    format!("sheet {sheet} is missing, so {expected} frozen rows were never read")
}

fn beyond(sheet: &str, index: usize, expected: usize) -> String {
    format!(
        "{} carries a row beyond the {expected}-row frozen projection",
        cell_at(sheet, index, 0)
    )
}

fn shape_note(shape: Shape, sheet: &str, expected: usize, findings: &mut Findings) {
    if shape.rows < expected {
        findings.note(format!(
            "sheet {sheet} holds {} rows where the frozen projection writes {expected}",
            shape.rows
        ));
    }
}

fn compare_row(row: &SparseRow, sheet: &str, expected: &[Expect], findings: &mut Findings) {
    for (column, cell) in expected.iter().enumerate() {
        compare(row, sheet, column, &value(cell), findings);
    }
    verify_width(row, sheet, expected.len(), findings);
}

fn value(expected: &Expect) -> Value {
    match expected {
        Expect::Text(text) => Value::Text(text.clone()),
        Expect::Number(number) => Value::Number(*number),
        Expect::Empty => Value::Empty,
    }
}

#[cfg(test)]
mod tests;
