use super::public_identity::{self, Expected};
use crate::report::ReportResult;
use census_domain::model::CanonicalAthlete;

use super::canonical::cell_at;
use super::expectations::Expectations;
use super::labels;
use super::labels::athlete_headers::Schema;
use super::read::{
    compare, verify_header, verify_width, Book, Budget, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;

mod cells;
mod membership;
mod rows;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    Verifier::new(expectations, findings)?.run(book)
}

struct Verifier<'a, 'd> {
    expectations: &'a Expectations<'d>,
    schema: Schema,
    headers: Vec<&'static str>,
    expected_ids: Vec<String>,
    ordered: Vec<&'a CanonicalAthlete>,
    printed: Vec<String>,
    seen: usize,
    findings: &'a mut Findings,
}

impl<'a, 'd> Verifier<'a, 'd> {
    fn new(expectations: &'a Expectations<'d>, findings: &'a mut Findings) -> ReportResult<Self> {
        let schema = Schema::of(expectations.dataset.semantic_policy()?);
        let ordered = ordered_athletes(expectations);
        let headers = public_identity::sheet_headers(expectations.dataset, schema.headers())
            .into_iter()
            .map(
                |header| match (expectations.dataset.selected_profile_id(), header) {
                    (Some(_), "Conflict Flag") => "Retained Source Conflict",
                    (Some(_), "Review Status") => "Identity Authority",
                    _ => header,
                },
            )
            .collect();
        let expected_ids: Vec<_> = ordered
            .iter()
            .map(|athlete| {
                Expected::of(expectations.dataset, athlete.id.as_str())
                    .map(|identity| identity.athlete_id)
            })
            .collect::<ReportResult<_>>()?;
        if expected_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != expected_ids.len()
        {
            return Err(crate::report::ReportError::Invariant {
                detail: "workbook athlete inventory repeats a public identity".into(),
            });
        }
        Ok(Self {
            expectations,
            schema,
            headers,
            expected_ids,
            ordered,
            printed: Vec::new(),
            seen: 0_usize,
            findings,
        })
    }

    fn run(mut self, book: &mut Book) -> ReportResult<()> {
        let budget = Budget {
            rows: EXCEL_ROWS_PER_SHEET,
            columns: EXCEL_COLUMNS_PER_SHEET,
        };
        let read = book.read(labels::ATHLETES, budget, |row| self.visit(row))?;
        if read.is_some() {
            self.findings.rows(self.seen);
        } else {
            self.findings.note(format!(
                "sheet {} is missing, so {} cohort athletes were never read",
                labels::ATHLETES,
                self.ordered.len()
            ));
        }
        self.membership();
        Ok(())
    }

    fn visit(&mut self, row: &SparseRow) {
        if self.header(row) {
            return;
        }
        let position = row.index().saturating_sub(1);
        let id = row.text(0).map_or("", |value| value).to_string();
        let expected = self.ordered.get(position).copied();
        if row.blank() {
            self.blank_row(row, expected);
            return;
        }
        if let Some(athlete) = expected {
            self.seen = self.seen.saturating_add(1);
            self.compare_row(row, athlete);
        }
        self.printed.push(id);
    }

    fn header(&mut self, row: &SparseRow) -> bool {
        verify_width(row, labels::ATHLETES, self.headers.len(), self.findings);
        if row.index() != 0 {
            return false;
        }
        verify_header(row, labels::ATHLETES, &self.headers, self.findings);
        true
    }

    fn blank_row(&mut self, row: &SparseRow, expected: Option<&CanonicalAthlete>) {
        if let Some(athlete) = expected {
            self.findings.note(format!(
                "{} is blank where athlete {} ({}) was expected",
                cell_at(labels::ATHLETES, row.index(), 0),
                athlete.canonical_name,
                athlete.id.as_str()
            ));
        }
    }

    fn compare_row(&mut self, row: &SparseRow, athlete: &CanonicalAthlete) {
        match self.values(athlete) {
            Ok(values) => values.iter().enumerate().for_each(|(column, expected)| {
                compare(row, labels::ATHLETES, column, expected, self.findings);
            }),
            Err(error) => self.findings.note(error.to_string()),
        }
    }

    fn membership(&mut self) {
        membership::verify(&self.expected_ids, &self.printed, self.findings);
    }
}

fn ordered_athletes<'a>(expectations: &'a Expectations<'_>) -> Vec<&'a CanonicalAthlete> {
    let mut ordered = expectations
        .derivation
        .athletes()
        .iter()
        .map(|athlete| {
            let school = athlete.school.as_str();
            let key = (
                expectations.school_state(school).to_string(),
                expectations.school_name(school).to_string(),
                athlete.canonical_name.clone(),
            );
            (athlete, key)
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    ordered.into_iter().map(|(athlete, _)| athlete).collect()
}

#[cfg(test)]
mod tests;
