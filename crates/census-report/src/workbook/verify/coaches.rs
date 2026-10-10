mod cells;

use crate::report::ReportResult;
use census_domain::model::CanonicalCoach;
use std::collections::BTreeMap;

use super::canonical::cell_at;
use super::expectations::Expectations;
use super::labels;
use super::read::{
    compare, verify_header, verify_width, Book, Budget, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;
use cells::{sport_label, values};

type SortKey = (String, String, String, String, String);

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    Verifier::new(expectations, findings).run(book)
}

struct Verifier<'a, 'd> {
    expectations: &'a Expectations<'d>,
    ordered: Vec<(&'a CanonicalCoach, SortKey)>,
    printed: Vec<String>,
    seen: usize,
    findings: &'a mut Findings,
}

impl<'a, 'd> Verifier<'a, 'd> {
    fn new(expectations: &'a Expectations<'d>, findings: &'a mut Findings) -> Self {
        Self {
            expectations,
            ordered: ordered(expectations),
            printed: Vec::new(),
            seen: 0,
            findings,
        }
    }

    fn run(mut self, book: &mut Book) -> ReportResult<()> {
        let budget = Budget {
            rows: EXCEL_ROWS_PER_SHEET,
            columns: EXCEL_COLUMNS_PER_SHEET,
        };
        match book.read(labels::COACHES, budget, |row: &SparseRow| self.visit(row))? {
            Some(_) => self.findings.rows(self.seen),
            None => self.findings.note(format!(
                "sheet {} is missing, so {} coach observations were never read",
                labels::COACHES,
                self.ordered.len()
            )),
        }
        membership(&self.ordered, &self.printed, self.findings);
        if self.seen != self.ordered.len() {
            self.findings.note(format!(
                "{} verified {} coach rows where the frozen dataset holds {} observations",
                labels::COACHES,
                self.seen,
                self.ordered.len()
            ));
        }
        Ok(())
    }

    fn visit(&mut self, row: &SparseRow) {
        if self.header(row) {
            return;
        }
        let position = row.index().saturating_sub(1);
        let id = row.text(6).map_or("", |value| value).to_string();
        if row.blank() {
            self.blank_row(row, position);
            return;
        }
        self.compare_admitted(row, position, &id);
        self.printed.push(id);
    }

    fn header(&mut self, row: &SparseRow) -> bool {
        verify_width(
            row,
            labels::COACHES,
            labels::COACH_HEADERS.len(),
            self.findings,
        );
        if row.index() != 0 {
            return false;
        }
        verify_header(row, labels::COACHES, &labels::COACH_HEADERS, self.findings);
        true
    }

    fn compare_admitted(&mut self, row: &SparseRow, position: usize, id: &str) {
        if let Some((coach, _)) = self.ordered.get(position) {
            self.seen = self.seen.saturating_add(1);
            for (column, expected) in values(self.expectations, coach).iter().enumerate() {
                compare(row, labels::COACHES, column, expected, self.findings);
            }
        } else {
            self.findings.note(format!(
                "{} carries an unexpected coach observation {id}",
                cell_at(labels::COACHES, row.index(), 6)
            ));
        }
    }

    fn blank_row(&mut self, row: &SparseRow, position: usize) {
        if let Some((coach, _)) = self.ordered.get(position) {
            self.findings.note(format!(
                "{} is blank where coach {} ({}) was expected",
                cell_at(labels::COACHES, row.index(), 6),
                coach.name,
                coach.id.as_str()
            ));
        }
    }
}

fn ordered<'a, 'd>(expectations: &'a Expectations<'d>) -> Vec<(&'a CanonicalCoach, SortKey)> {
    let mut ordered: Vec<(&CanonicalCoach, SortKey)> = expectations
        .coach_claims
        .iter()
        .map(|coach| (coach, sort_key(expectations, coach)))
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    ordered
}

fn sort_key(expectations: &Expectations<'_>, coach: &CanonicalCoach) -> SortKey {
    let school = coach.school.as_str();
    (
        expectations.school_state(school).to_string(),
        expectations.school_name(school).to_string(),
        sport_label(coach),
        coach.role.stable_key().to_string(),
        coach.name.clone(),
    )
}

fn membership(ordered: &[(&CanonicalCoach, SortKey)], printed: &[String], findings: &mut Findings) {
    let expected = observation_counts(ordered);
    let actual = printed_counts(printed);
    report_excess(&expected, &actual, findings);
    report_missing(&expected, &actual, findings);
}

fn observation_counts<'a>(ordered: &[(&'a CanonicalCoach, SortKey)]) -> BTreeMap<&'a str, usize> {
    let mut expected: BTreeMap<&str, usize> = BTreeMap::new();
    for (coach, _) in ordered {
        let count = expected.entry(coach.id.as_str()).or_insert(0);
        *count = count.saturating_add(1);
    }
    expected
}

fn printed_counts(printed: &[String]) -> BTreeMap<&str, usize> {
    let mut actual: BTreeMap<&str, usize> = BTreeMap::new();
    for id in printed {
        let count = actual.entry(id.as_str()).or_insert(0);
        *count = count.saturating_add(1);
    }
    actual
}

fn report_excess(
    expected: &BTreeMap<&str, usize>,
    actual: &BTreeMap<&str, usize>,
    findings: &mut Findings,
) {
    for (id, count) in actual {
        let held = expected
            .get(id)
            .copied()
            .map_or(Default::default(), core::convert::identity);
        if held < *count {
            let message = if expected.contains_key(id) {
                format!(
                    "coach {id} is printed {count} times where the frozen dataset holds it {held} \
                     time(s)"
                )
            } else {
                format!("coach {id} is printed but absent from the frozen coach observations")
            };
            findings.note(message);
        }
    }
}

fn report_missing(
    expected: &BTreeMap<&str, usize>,
    actual: &BTreeMap<&str, usize>,
    findings: &mut Findings,
) {
    for (id, count) in expected {
        let found = actual
            .get(id)
            .copied()
            .map_or(Default::default(), core::convert::identity);
        if found < *count {
            findings.note(format!(
                "coach {id} holds {count} observation(s) in the frozen dataset but is printed \
                 {found} time(s)"
            ));
        }
    }
}

#[cfg(test)]
mod tests;
