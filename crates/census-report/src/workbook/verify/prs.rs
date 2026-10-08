use crate::bests::{Conflict, SharedSelection};
use crate::report::ReportResult;

use super::canonical::Value;
use super::expectations::Expectations;
use super::labels;
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
    let prs = &expectations.bests;
    let mut seen = 0_usize;
    let mut visit = |row: &SparseRow| {
        visit_row(row, prs, &mut seen, findings);
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

fn visit_row(row: &SparseRow, prs: &[SharedSelection], seen: &mut usize, findings: &mut Findings) {
    if header(row, findings) {
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
    for (column, expected) in values(pr).enumerate() {
        compare(row, labels::PRS, column, &expected, findings);
    }
}

fn header(row: &SparseRow, findings: &mut Findings) -> bool {
    verify_width(row, labels::PRS, labels::PR_HEADERS.len(), findings);
    if row.index() != 0 {
        return false;
    }
    verify_header(row, labels::PRS, &labels::PR_HEADERS, findings);
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

fn values(pr: &SharedSelection) -> impl Iterator<Item = Value> + use<> {
    identity_values(pr)
        .into_iter()
        .chain(result_values(pr))
        .chain(context_values(pr))
}

fn identity_values(pr: &SharedSelection) -> [Value; 9] {
    [
        Value::text(pr.athlete_id().as_str()),
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

fn context_values(pr: &SharedSelection) -> [Value; 7] {
    [
        Value::text(pr.key.surface.label()),
        Value::text(pr.key.wind_class.label()),
        Value::text(pr.key.timing.label()),
        Value::text(pr.source.performance_id.as_str()),
        Value::text(pr.meet.meet_id.as_str()),
        Value::text(&pr.source.source_key),
        event_context(pr),
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
