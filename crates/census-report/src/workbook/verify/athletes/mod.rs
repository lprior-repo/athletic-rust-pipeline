use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, IdentityStatus, Sport};
use std::collections::BTreeMap;

use super::canonical::{cell_at, Value};
use super::expectations::Expectations;
use super::labels;
use super::reach;
use super::read::{
    compare, verify_header, verify_width, Book, Budget, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;

mod cells;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    Verifier::new(expectations, findings).run(book)
}

struct Verifier<'a, 'd> {
    expectations: &'a Expectations<'d>,
    ordered: Vec<&'a CanonicalAthlete>,
    printed: Vec<String>,
    seen: usize,
    findings: &'a mut Findings,
}

impl<'a, 'd> Verifier<'a, 'd> {
    fn new(expectations: &'a Expectations<'d>, findings: &'a mut Findings) -> Self {
        let mut ordered: Vec<(&CanonicalAthlete, (String, String, String))> = expectations
            .derivation
            .athletes()
            .iter()
            .map(|athlete| {
                let school = athlete.school.as_str();
                (
                    athlete,
                    (
                        expectations.school_state(school).to_string(),
                        expectations.school_name(school).to_string(),
                        athlete.canonical_name.clone(),
                    ),
                )
            })
            .collect();
        ordered.sort_by(|left, right| left.1.cmp(&right.1));
        Self {
            expectations,
            ordered: ordered.into_iter().map(|(athlete, _)| athlete).collect(),
            printed: Vec::new(),
            seen: 0_usize,
            findings,
        }
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
        verify_width(
            row,
            labels::ATHLETES,
            labels::ATHLETE_HEADERS.len(),
            self.findings,
        );
        if row.index() == 0 {
            verify_header(
                row,
                labels::ATHLETES,
                &labels::ATHLETE_HEADERS,
                self.findings,
            );
            return;
        }
        let position = row.index().saturating_sub(1);
        let id = row.text(0).map_or("", |value| value).to_string();
        let expected = self.ordered.get(position).copied();
        if row.blank() {
            if let Some(athlete) = expected {
                self.findings.note(format!(
                    "{} is blank where athlete {} ({}) was expected",
                    cell_at(labels::ATHLETES, row.index(), 0),
                    athlete.canonical_name,
                    athlete.id.as_str()
                ));
            }
            return;
        }
        if let Some(athlete) = expected {
            self.seen = self.seen.saturating_add(1);
            self.compare_row(row, athlete);
        }
        self.printed.push(id);
    }

    fn compare_row(&mut self, row: &SparseRow, athlete: &CanonicalAthlete) {
        for (column, expected) in self.values(athlete).iter().enumerate() {
            compare(row, labels::ATHLETES, column, expected, self.findings);
        }
    }

    fn values(&mut self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let prs: Vec<&SharedSelection> = self.expectations.prs_of(athlete.id.as_str()).collect();
        let mut cells: Vec<Value> = Vec::with_capacity(labels::ATHLETE_HEADERS.len());
        cells.extend(self.identity(athlete));
        cells.push(cells::flag(athlete.sports.iter().any(|sport| {
            matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack)
        })));
        cells.push(cells::flag(athlete.sports.contains(&Sport::CrossCountry)));
        cells.push(cells::flag(athlete.sports.contains(&Sport::IndoorTrack)));
        cells.push(cells::flag(athlete.sports.contains(&Sport::OutdoorTrack)));
        cells.push(cells::event_list(&prs));
        cells.push(cells::headline(&prs));
        cells.extend(cells::pr_events(&prs));
        cells.extend(self.tally_cells(athlete));
        let reach = reach::Reach::of(self.expectations, athlete);
        cells.push(reach.track_names);
        cells.push(reach.track_emails);
        cells.push(reach.cross_country_name);
        cells.push(reach.cross_country_email);
        cells.push(reach.professional);
        cells.push(reach.director_name);
        cells.push(reach.director_email);
        cells.push(Value::optional(
            self.expectations.athletics_url(athlete.school.as_str()),
        ));
        cells.push(Value::Empty);
        cells.push(Value::Empty);
        cells.push(reach.all_emails);
        cells.push(reach.preferred_name);
        cells.push(reach.preferred_role);
        cells.push(reach.preferred_email);
        cells.push(reach.preferred_state);
        let profiles = cells::profiles_of(athlete);
        cells.push(Value::optional(profiles.athletic_net.as_deref()));
        cells.push(Value::optional(profiles.milesplit.as_deref()));
        cells.push(Value::text(profiles.other.join("; ")));
        cells.push(Value::count(cells::source_count(athlete)));
        cells.extend(self.status_cells(athlete));
        match self.expectations.postal.get(athlete.id.as_str()) {
            Some(fields) => cells.extend(fields.iter().map(Value::text)),
            None => {
                self.findings.note(format!(
                    "athlete {} has no frozen postal projection",
                    athlete.id
                ));
                cells.extend(std::iter::repeat_n(Value::Empty, 12));
            }
        }
        cells
    }

    fn identity(&self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let school = athlete.school.as_str();
        vec![
            Value::text(athlete.id.as_str()),
            Value::text(&athlete.canonical_name),
            Value::text(athlete.gender.stable_key()),
            Value::integer(i64::from(athlete.grad_year.get())),
            Value::optional(
                cells::newest_observation(athlete).map(|grade| grade.school_year.short()),
            ),
            Value::text(self.expectations.school_state(school)),
            Value::text(self.expectations.school_name(school)),
            Value::text(self.expectations.school_id(school)),
            Value::text(self.expectations.school_city(school)),
        ]
    }

    fn tally_cells(&self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let tally = self.expectations.tally(athlete.id.as_str());
        vec![
            Value::count(tally.map_or(0, |tally| tally.performances)),
            Value::count(tally.map_or(0, |tally| tally.meets.len())),
        ]
    }

    fn status_cells(&mut self, athlete: &CanonicalAthlete) -> Vec<Value> {
        let status = self
            .expectations
            .dataset
            .identities()
            .status(athlete.id.as_str());
        let Ok(status) = status else {
            self.findings.note(format!(
                "athlete {} has no identity decision in the frozen archive, so its status, \
                 coverage, conflict and review columns are unverified",
                athlete.id.as_str()
            ));
            return vec![
                Value::Empty,
                self.coverage_state(athlete),
                Value::Empty,
                Value::Empty,
            ];
        };
        vec![
            Value::text(status.as_str()),
            self.coverage_state(athlete),
            Value::flagged(status == IdentityStatus::RetainedConflict),
            Value::text(if status == IdentityStatus::Verified {
                "verified"
            } else {
                "review"
            }),
        ]
    }

    fn coverage_state(&self, athlete: &CanonicalAthlete) -> Value {
        let performances = self
            .expectations
            .tally(athlete.id.as_str())
            .is_some_and(|tally| tally.performances > 0);
        let prs = self
            .expectations
            .prs_of(athlete.id.as_str())
            .next()
            .is_some();
        match (performances, prs) {
            (true, true) => Value::text("pr"),
            (true, false) => Value::text("performance"),
            (false, _) => Value::text("identity-only"),
        }
    }

    fn membership(&mut self) {
        let mut expected: BTreeMap<&str, usize> = BTreeMap::new();
        for athlete in &self.ordered {
            let count = expected.entry(athlete.id.as_str()).or_insert(0);
            *count = count.saturating_add(1);
        }
        let mut actual: BTreeMap<&str, usize> = BTreeMap::new();
        for id in &self.printed {
            let count = actual.entry(id.as_str()).or_insert(0);
            *count = count.saturating_add(1);
        }
        for (id, count) in &actual {
            let held = expected
                .get(id)
                .copied()
                .map_or(Default::default(), core::convert::identity);
            if held < *count {
                let message = if expected.contains_key(id) {
                    format!(
                        "athlete {id} is printed {count} times where the frozen cohort holds it \
                         {held} time(s)"
                    )
                } else {
                    format!("athlete {id} is printed but absent from the frozen cohort")
                };
                self.findings.note(message);
            }
        }
        for (id, count) in &expected {
            let found = actual
                .get(id)
                .copied()
                .map_or(Default::default(), core::convert::identity);
            if found < *count {
                self.findings.note(format!(
                    "athlete {id} holds {count} cohort row(s) in the frozen dataset but is printed \
                     {found} time(s)"
                ));
            }
        }
    }
}
