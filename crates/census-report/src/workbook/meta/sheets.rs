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

pub(super) struct Inputs<'a, 'd> {
    facts: &'a RunFacts<'d>,
    rows: &'a StoreRows<'d>,
    conflicts: &'a [Family],
    review: &'a [Family],
    index: &'a SubjectIndex<'d>,
}

impl<'a, 'd> Inputs<'a, 'd> {
    pub(super) fn of(
        facts: &'a RunFacts<'d>,
        rows: &'a StoreRows<'d>,
        conflicts: &'a [Family],
        review: &'a [Family],
        index: &'a SubjectIndex<'d>,
    ) -> Self {
        Self {
            facts,
            rows,
            conflicts,
            review,
            index,
        }
    }

    fn schools(&self) -> ReportResult<Sheet> {
        let cells = built("Schools", || schools_sheet(self.rows.schools))?;
        Ok(("Schools", cells, &SCHOOL_WIDTHS, true))
    }

    fn meets(&self) -> Sheet {
        let cells = built("Meets", || meets_sheet(self.rows.meets));
        ("Meets", cells, &MEET_WIDTHS, true)
    }

    fn sources(&self) -> ReportResult<Sheet> {
        let cells = built("Sources", || sources_sheet(self.facts.all_sources))?;
        Ok(("Sources", cells, &SOURCE_WIDTHS, true))
    }

    fn coverage(&self) -> ReportResult<Sheet> {
        let cells = built("Coverage", || {
            coverage_sheet(self.facts.population.dataset())
        })?;
        Ok(("Coverage", cells, &COVERAGE_WIDTHS, true))
    }

    fn conflicts(&self) -> Sheet {
        let cells = built("Conflicts", || conflicts_sheet(self.conflicts, self.index));
        ("Conflicts", cells, &CONFLICT_WIDTHS, true)
    }

    fn review(&self) -> Sheet {
        let cells = built("Review", || {
            review_sheet(self.review, self.rows, self.index)
        });
        ("Review", cells, &REVIEW_WIDTHS, true)
    }
}

pub(super) fn meta_sheets(
    inputs: &Inputs<'_, '_>,
    metrics: Vec<Vec<Cell>>,
) -> ReportResult<Vec<Sheet>> {
    Ok(vec![
        inputs.schools()?,
        inputs.meets(),
        inputs.sources()?,
        inputs.coverage()?,
        inputs.conflicts(),
        inputs.review(),
        ("Run Metrics", metrics, &METRIC_WIDTHS, false),
    ])
}
