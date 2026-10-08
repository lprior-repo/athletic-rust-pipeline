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
        verify_width(row, labels::PRS, labels::PR_HEADERS.len(), findings);
        if row.index() == 0 {
            verify_header(row, labels::PRS, &labels::PR_HEADERS, findings);
            return;
        }
        let position = row.index().saturating_sub(1);
        let Some(pr) = prs.get(position) else {
            findings.note(format!(
                "{} carries an unexpected PR row whose athlete reads {:?}",
                coordinate(labels::PRS, row.index(), 0),
                row.text(0).map_or("", |value| value)
            ));
            return;
        };
        if row.blank() {
            findings.note(format!(
                "{} of {} is blank where the shared reduction selected PR row {} for athlete {}",
                coordinate(labels::PRS, row.index(), 0),
                labels::PRS,
                position,
                pr.athlete_id().as_str()
            ));
            return;
        }
        seen = seen.saturating_add(1);
        for (column, expected) in values(pr).iter().enumerate() {
            compare(row, labels::PRS, column, expected, findings);
        }
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

fn values(pr: &SharedSelection) -> [Value; 26] {
    [
        Value::text(pr.athlete_id().as_str()),
        Value::text(&pr.athlete),
        Value::text(pr.gender.stable_key()),
        Value::optional(pr.school.as_deref()),
        Value::text(pr.athlete_state.code()),
        Value::integer(i64::from(pr.grad_year)),
        Value::text(pr.sport()),
        Value::text(pr.key.event_kind.stable_key()),
        Value::text(pr.season()),
        Value::text(pr.mark_text()),
        pr.normalized.map_or(Value::Empty, Value::decimal),
        Value::optional(pr.unit()),
        pr.wind_mps.map_or(Value::Empty, Value::decimal),
        Value::text(&pr.date),
        Value::text(&pr.meet),
        pr.place
            .map_or(Value::Empty, |place| Value::count(usize::from(place))),
        Value::text(&pr.result_url),
        Value::count(pr.population.sources),
        conflict(&pr.conflicts),
        Value::text(pr.key.surface.label()),
        Value::text(pr.key.wind_class.label()),
        Value::text(pr.key.timing.label()),
        Value::text(pr.performance_id.as_str()),
        Value::text(pr.meet_id.as_str()),
        Value::text(&pr.source_key),
        pr.key
            .context
            .as_ref()
            .map_or(Value::Empty, |id| Value::text(id.as_str())),
    ]
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
        None => {
            findings.note(format!(
                "sheet {} is missing, so {} selected PR rows were never read",
                labels::PRS,
                prs.len()
            ));
        }
    }
}

fn coordinate(sheet: &str, row: usize, column: usize) -> String {
    format!("{sheet}!{}", super::canonical::cell_reference(row, column))
}
