use crate::export::coach_source;
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalCoach, CoachRole, CoachTenure, SchoolYear, TenureAssessmentError,
};
use std::collections::BTreeMap;

use super::canonical::{cell_at, Value};
use super::expectations::Expectations;
use super::labels;
use super::read::{
    compare, verify_header, verify_width, Book, Budget, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;

type SortKey = (String, String, String, String, String);

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let ordered = ordered(expectations);
    let mut printed: Vec<String> = Vec::new();
    let mut seen = 0_usize;
    let mut visit = |row: &SparseRow| {
        verify_width(row, labels::COACHES, labels::COACH_HEADERS.len(), findings);
        if row.index() == 0 {
            verify_header(row, labels::COACHES, &labels::COACH_HEADERS, findings);
            return;
        }
        let position = row.index().saturating_sub(1);
        let id = row.text(6).unwrap_or("").to_string();
        let expected = ordered.get(position);
        if row.blank() {
            if let Some((coach, _)) = expected {
                findings.note(format!(
                    "{} is blank where coach {} ({}) was expected",
                    cell_at(labels::COACHES, row.index(), 6),
                    coach.name,
                    coach.id.as_str()
                ));
            }
            return;
        }
        if let Some((coach, _)) = expected.filter(|(coach, _)| coach.id.as_str() == id) {
            seen = seen.saturating_add(1);
            for (column, expected) in values(expectations, coach).iter().enumerate() {
                compare(row, labels::COACHES, column, expected, findings);
            }
        }
        printed.push(id);
    };
    let budget = Budget {
        rows: EXCEL_ROWS_PER_SHEET,
        columns: EXCEL_COLUMNS_PER_SHEET,
    };
    match book.read(labels::COACHES, budget, &mut visit)? {
        Some(_) => findings.rows(seen),
        None => findings.note(format!(
            "sheet {} is missing, so {} coach observations were never read",
            labels::COACHES,
            ordered.len()
        )),
    }
    membership(&ordered, &printed, findings);
    Ok(())
}

fn ordered<'a, 'd>(expectations: &'a Expectations<'d>) -> Vec<(&'a CanonicalCoach, SortKey)> {
    let mut ordered: Vec<(&CanonicalCoach, SortKey)> = expectations
        .derivation
        .coach_observations()
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

fn values(expectations: &Expectations<'_>, coach: &CanonicalCoach) -> [Value; 18] {
    let school = coach.school.as_str();
    let director = expectations
        .contacts
        .get(school)
        .and_then(|facts| facts.director());
    let (source_url, observed_on) = coach_source(coach);
    [
        Value::text(school),
        Value::text(expectations.school_name(school)),
        Value::text(expectations.school_city(school)),
        Value::text(expectations.school_state(school)),
        Value::text(sport_label(coach)),
        Value::text(&coach.name),
        Value::text(coach.id.as_str()),
        Value::text(coach.gender.stable_key()),
        Value::text(coach.role.stable_key()),
        Value::optional(coach.professional_email.as_deref()),
        Value::optional(coach.personal_email.as_deref()),
        Value::optional(coach.phone.as_deref()),
        Value::optional(director.map(|row| row.name.as_str())),
        Value::optional(director.and_then(|row| row.email.as_deref())),
        Value::optional(source_url),
        Value::optional(observed_on),
        Value::text(tenure_label(coach, expectations.school_year)),
        Value::text(expectations.school_year.short()),
    ]
}

fn tenure_label(coach: &CanonicalCoach, school_year: SchoolYear) -> &'static str {
    match coach.tenure_state(school_year) {
        Ok(CoachTenure::Current { .. }) => "current_declared",
        Ok(CoachTenure::Former { .. }) => "former_declared",
        Ok(CoachTenure::Unknown) => "unknown",
        Err(TenureAssessmentError::Conflict) => "tenure_conflict",
        Err(TenureAssessmentError::InvalidEvidence { .. }) => "invalid_tenure_evidence",
    }
}

fn sport_label(coach: &CanonicalCoach) -> String {
    match (coach.sport, coach.role) {
        (Some(sport), _) => sport.stable_key().to_owned(),
        (None, CoachRole::AthleticDirector) => "school_wide".to_owned(),
        (None, _) => "unknown".to_owned(),
    }
}

fn membership(ordered: &[(&CanonicalCoach, SortKey)], printed: &[String], findings: &mut Findings) {
    let mut expected: BTreeMap<&str, usize> = BTreeMap::new();
    for (coach, _) in ordered {
        let count = expected.entry(coach.id.as_str()).or_insert(0);
        *count = count.saturating_add(1);
    }
    let mut actual: BTreeMap<&str, usize> = BTreeMap::new();
    for id in printed {
        let count = actual.entry(id.as_str()).or_insert(0);
        *count = count.saturating_add(1);
    }
    for (id, count) in &actual {
        let held = expected.get(id).copied().unwrap_or_default();
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
    for (id, count) in &expected {
        let found = actual.get(id).copied().unwrap_or_default();
        if found < *count {
            findings.note(format!(
                "coach {id} holds {count} observation(s) in the frozen dataset but is printed \
                 {found} time(s)"
            ));
        }
    }
}
