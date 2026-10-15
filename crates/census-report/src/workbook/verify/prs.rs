use crate::bests::{Conflict, SharedSelection};
use crate::report::ReportResult;

use super::canonical::Value;
use super::current_school::AffiliationExpectations;
use super::expectations::Expectations;
use super::labels;
use super::public_identity::{self, Expected};
use super::read::{
    compare, verify_header, verify_width, Book, Budget, Shape, SparseRow, EXCEL_COLUMNS_PER_SHEET,
    EXCEL_ROWS_PER_SHEET,
};
use super::report::Findings;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let schools = if expectations.dataset.selected_profile_id().is_some() {
        Some(AffiliationExpectations::of(expectations.dataset)?)
    } else {
        None
    };
    let prs = &expectations.bests;
    let mut seen = 0_usize;
    let mut visit = |row: &SparseRow| {
        visit_row(row, expectations, schools.as_ref(), &mut seen, findings);
    };
    let budget = Budget {
        rows: EXCEL_ROWS_PER_SHEET,
        columns: EXCEL_COLUMNS_PER_SHEET,
    };
    let shape = book.read(labels::PRS, budget, &mut visit)?;
    report_prs_read_result(shape, prs, seen, findings);
    if seen != prs.len() {
        findings.note(format!(
            "{} verified {seen} PR rows where the shared reduction selected {}",
            labels::PRS,
            prs.len()
        ));
    }
    Ok(())
}

fn visit_row(
    row: &SparseRow,
    expectations: &Expectations<'_>,
    schools: Option<&AffiliationExpectations<'_>>,
    seen: &mut usize,
    findings: &mut Findings,
) {
    let prs = &expectations.bests;
    if header(row, expectations, findings) {
        return;
    }
    let Some(pr) = expected_pr(row, prs, findings) else {
        return;
    };
    if row.blank() {
        blank_pr(row, pr, findings);
        return;
    }
    *seen = seen.saturating_add(1);
    match values(pr, expectations, schools) {
        Ok(values) => values.enumerate().for_each(|(column, expected)| {
            compare(row, labels::PRS, column, &expected, findings);
        }),
        Err(error) => findings.note(error.to_string()),
    }
}

fn header(row: &SparseRow, expectations: &Expectations<'_>, findings: &mut Findings) -> bool {
    let mut headers = public_identity::sheet_headers(expectations.dataset, &labels::PR_HEADERS);
    if expectations.dataset.selected_profile_id().is_some() {
        for header in &mut headers {
            *header = match *header {
                "School" => "School at performance",
                "State" => "State at performance",
                other => other,
            };
        }
        headers.extend(["Current school", "Current state", "Current school as-of"]);
    }
    verify_width(row, labels::PRS, headers.len(), findings);
    if row.index() != 0 {
        return false;
    }
    verify_header(row, labels::PRS, &headers, findings);
    true
}

fn expected_pr<'a>(
    row: &SparseRow,
    prs: &'a [SharedSelection],
    findings: &mut Findings,
) -> Option<&'a SharedSelection> {
    let position = row.index().saturating_sub(1);
    let expected = prs.get(position);
    if expected.is_none() {
        findings.note(format!(
            "{} carries an unexpected PR row whose athlete reads {:?}",
            coordinate(labels::PRS, row.index(), 0),
            row.text(0).map_or("", |value| value)
        ));
    }
    expected
}

fn blank_pr(row: &SparseRow, pr: &SharedSelection, findings: &mut Findings) {
    findings.note(format!(
        "{} of {} is blank where the shared reduction selected PR row {} for athlete {}",
        coordinate(labels::PRS, row.index(), 0),
        labels::PRS,
        row.index().saturating_sub(1),
        pr.athlete_id().as_str()
    ));
}

fn values(
    pr: &SharedSelection,
    expectations: &Expectations<'_>,
    schools: Option<&AffiliationExpectations<'_>>,
) -> ReportResult<impl Iterator<Item = Value> + use<>> {
    let identity = Expected::of(expectations.dataset, pr.athlete_id().as_str())?;
    let current = schools
        .map(|schools| schools.values(pr.athlete_id().as_str()))
        .transpose()?;
    let current = current.map(|[_, name, state, as_of]| [name, state, as_of].map(Value::text));
    Ok(identity_values(pr, &identity.athlete_id)
        .into_iter()
        .chain(result_values(pr))
        .chain(context_values(pr))
        .chain(identity.audit.into_iter().flatten().map(Value::text))
        .chain(current.into_iter().flatten()))
}

fn identity_values(pr: &SharedSelection, athlete_id: &str) -> [Value; 9] {
    [
        Value::text(athlete_id),
        Value::text(&pr.athlete.name),
        Value::text(pr.athlete.gender.stable_key()),
        Value::optional(pr.athlete.school.as_deref()),
        Value::text(pr.athlete.athlete_state.code()),
        Value::integer(i64::from(pr.athlete.grad_year)),
        Value::text(pr.sport()),
        Value::text(pr.key.event_kind.stable_key()),
        Value::text(pr.season()),
    ]
}

fn result_values(pr: &SharedSelection) -> [Value; 10] {
    [
        Value::text(pr.mark_text()),
        pr.result.normalized.map_or(Value::Empty, Value::decimal),
        Value::optional(pr.unit()),
        pr.result.wind_mps.map_or(Value::Empty, Value::decimal),
        Value::text(&pr.meet.date),
        Value::text(&pr.meet.name),
        pr.result
            .place
            .map_or(Value::Empty, |place| Value::count(usize::from(place))),
        Value::text(&pr.source.result_url),
        Value::count(pr.population.sources),
        conflict(&pr.conflicts),
    ]
}
fn context_values(pr: &SharedSelection) -> [Value; 10] {
    [
        Value::text(pr.key.surface.label()),
        Value::text(pr.key.wind_class.label()),
        Value::text(pr.key.timing.label()),
        Value::text(pr.source.performance_id.as_str()),
        Value::text(pr.meet.meet_id.as_str()),
        Value::text(&pr.source.source_key),
        event_context(pr),
        published_mark_value(pr),
        conversion_basis_value(pr),
        conversion_note_value(pr),
    ]
}

fn event_context(pr: &SharedSelection) -> Value {
    match super::context::cross_country(pr) {
        Some(context) => Value::text(context),
        None => pr
            .key
            .context
            .as_ref()
            .map_or(Value::Empty, |id| Value::text(id.as_str())),
    }
}

fn conflict(conflicts: &[Conflict]) -> Value {
    if conflicts.is_empty() {
        return Value::Empty;
    }
    let mut text = String::new();
    for (index, conflict) in conflicts.iter().enumerate() {
        if index != 0 {
            text.push_str("; ");
        }
        text.push_str(&conflict.meet);
        text.push_str(": ");
        for (index, mark) in conflict.marks.iter().enumerate() {
            if index != 0 {
                text.push_str(" | ");
            }
            text.push_str(mark);
        }
    }
    Value::text(text)
}
fn published_mark_value(pr: &SharedSelection) -> Value {
    match &pr.result.original_mark {
        Some(original) => Value::text(crate::bests::mark_text(original)),
        None => Value::Empty,
    }
}

fn conversion_basis_value(pr: &SharedSelection) -> Value {
    match &pr.result.adjustment_kind {
        Some(_) => match pr.result.adjustment_basis {
            Some(basis) => Value::text(basis.label()),
            None => Value::text("unknown"),
        },
        None => Value::Empty,
    }
}

fn conversion_note_value(pr: &SharedSelection) -> Value {
    match &pr.result.adjustment_note {
        Some(note) => Value::text(note),
        None => Value::Empty,
    }
}

fn report_prs_read_result(
    shape: Option<Shape>,
    prs: &[SharedSelection],
    seen: usize,
    findings: &mut Findings,
) {
    match shape {
        Some(shape) => {
            findings.rows(seen);
            let written = shape.rows.saturating_sub(1);
            report_short_sheet(written, prs, findings);
        }
        None => {
            findings.note(format!(
                "sheet {} is missing, so {} selected PR rows were never read",
                labels::PRS,
                prs.len()
            ));
        }
    }
}

fn report_short_sheet(written: usize, prs: &[SharedSelection], findings: &mut Findings) {
    if written < prs.len() {
        findings.note(format!(
            "{} holds {written} PR rows where the shared reduction selected {}; the first \
             unread athlete is {}",
            labels::PRS,
            prs.len(),
            prs.get(written)
                .map(|pr| pr.athlete_id().as_str())
                .map_or("(none)", |value| value)
        ));
    }
}

fn coordinate(sheet: &str, row: usize, column: usize) -> String {
    format!("{sheet}!{}", super::canonical::cell_reference(row, column))
}

#[cfg(test)]
mod tests;
