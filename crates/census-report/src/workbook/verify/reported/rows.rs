use super::super::canonical::{cell_at, Value};
use super::super::read::{compare, verify_header, verify_width, Book, Budget, Shape};
use super::super::report::Findings;
use crate::report::ReportResult;

pub(in crate::workbook::verify) fn verify(
    book: &mut Book,
    sheet: &str,
    headers: &[&str],
    total: usize,
    mut expected: impl FnMut(usize, &mut Findings) -> ReportResult<Vec<Value>>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let expected_rows = total
        .checked_add(1)
        .ok_or_else(|| super::invalid("mixed worksheet row count overflow"))?;
    let mut seen = 0usize;
    let mut failure = None;
    let shape = book.read_family(sheet, Budget::family(total)?, |row| {
        verify_width(row, sheet, headers.len(), findings);
        if row.index() == 0 {
            verify_header(row, sheet, headers, findings);
            return;
        }
        let Some(position) = row
            .index()
            .checked_sub(1)
            .filter(|position| *position < total)
        else {
            findings.note(format!(
                "{} is an unexpected mixed worksheet row",
                cell_at(sheet, row.index(), 0)
            ));
            return;
        };
        let Some(next) = seen.checked_add(1) else {
            failure = Some(super::invalid("mixed worksheet seen count overflow"));
            return;
        };
        seen = next;
        match expected(position, findings) {
            Ok(values) if values.len() == headers.len() => {
                for (column, value) in values.iter().enumerate() {
                    compare(row, sheet, column, value, findings);
                }
            }
            Ok(_) => {
                failure = Some(super::invalid(
                    "mixed worksheet expected width differs from header",
                ));
            }
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error);
                }
            }
        }
    })?;
    if let Some(error) = failure {
        return Err(error);
    }
    compare_extent(sheet, shape, expected_rows, findings);
    record_count(sheet, seen, total, findings);
    Ok(())
}

fn compare_extent(
    sheet: &str,
    shape: Option<Shape>,
    expected_rows: usize,
    findings: &mut Findings,
) {
    match shape {
        Some(shape) if shape.rows != expected_rows => {
            findings.note(format!(
                "{sheet} holds {} logical rows; expected {expected_rows}",
                shape.rows
            ));
        }
        None => findings.note(format!("sheet {sheet} is missing")),
        Some(_) => {}
    }
}

fn record_count(sheet: &str, seen: usize, total: usize, findings: &mut Findings) {
    if seen != total {
        findings.note(format!(
            "{sheet} read {seen} mixed data rows; expected {total}"
        ));
    }
    findings.rows(seen);
}
