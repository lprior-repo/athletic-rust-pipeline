use super::coverage::{coverage_sheet, COVERAGE_WIDTHS};
use super::inventory::{meets_sheet, MEET_WIDTHS};
use super::metrics::METRIC_WIDTHS;
use super::queues::{conflicts_sheet, review_sheet, CONFLICT_WIDTHS, REVIEW_WIDTHS};
use super::schools::{schools_sheet, SCHOOL_WIDTHS};
use super::sources::{sources_sheet, SOURCE_WIDTHS};
use super::{millis, Family, ReportResult, RunFacts, Sheet, StoreRows, SubjectIndex};
use crate::workbook::cells::Cell;
use std::time::Instant;

fn built<T>(sheet: &'static str, build: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = build();
    tracing::info!(sheet, ms = millis(started), "workbook sheet built");
    value
}

pub(super) fn meta_sheets(
    facts: &RunFacts<'_>,
    rows: &StoreRows<'_>,
    conflicts: &[Family],
    review: &[Family],
    index: &SubjectIndex<'_>,
    metrics: Vec<Vec<Cell>>,
) -> ReportResult<Vec<Sheet>> {
    let mut sheets: Vec<Sheet> = Vec::with_capacity(7);
    sheets.push((
        "Schools",
        built("Schools", || schools_sheet(&rows.schools))?,
        &SCHOOL_WIDTHS,
        true,
    ));
    sheets.push((
        "Meets",
        built("Meets", || meets_sheet(&rows.meets)),
        &MEET_WIDTHS,
        true,
    ));
    sheets.push((
        "Sources",
        built("Sources", || sources_sheet(facts.all_sources))?,
        &SOURCE_WIDTHS,
        true,
    ));
    sheets.push((
        "Coverage",
        built("Coverage", || coverage_sheet(facts.population.dataset()))?,
        &COVERAGE_WIDTHS,
        true,
    ));
    sheets.push((
        "Conflicts",
        built("Conflicts", || conflicts_sheet(conflicts, index)),
        &CONFLICT_WIDTHS,
        true,
    ));
    sheets.push((
        "Review",
        built("Review", || review_sheet(review, rows, index)),
        &REVIEW_WIDTHS,
        true,
    ));
    sheets.push(("Run Metrics", metrics, &METRIC_WIDTHS, false));
    Ok(sheets)
}
