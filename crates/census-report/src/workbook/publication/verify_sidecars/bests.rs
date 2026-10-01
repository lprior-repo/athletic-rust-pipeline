use std::path::Path;

use crate::bests as selections;
use crate::report::{ReportError, ReportResult};

use super::cells::Cell;
use super::input::Inputs;
use super::read::{self, Limits};
use super::{cohort, defect, diff};

const HEADERS: [&str; 24] = [
    "athlete_id",
    "name",
    "school",
    "state",
    "grad_year",
    "gender",
    "sport",
    "event",
    "surface",
    "wind_class",
    "timing_class",
    "best_mark",
    "best_value",
    "measure",
    "date",
    "meet",
    "place",
    "wind_mps",
    "timing",
    "marks_in_event",
    "profile_url",
    "performance_id",
    "source_athlete",
    "source_key",
];

const LIMITS: Limits = Limits {
    bytes: 2 * 1024 * 1024 * 1024,
    records: 4_000_000,
};

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let rows = selections::build_from_dataset(
        inputs.dataset,
        &selections::Options {
            scope: inputs.options.scope,
            grad_year: inputs.options.grad_year,
            limit: inputs.options.limit,
        },
    );
    let cohort = cohort(inputs.options.grad_year);
    verify_csv(&directory.join(format!("best-results-{cohort}.csv")), &rows)?;
    verify_jsonl(
        &directory.join(format!("best-results-{cohort}.jsonl")),
        &rows,
    )
}

fn verify_csv(path: &Path, rows: &[selections::SharedSelection]) -> ReportResult<()> {
    let mut position = 0usize;
    let total = read::csv(path, LIMITS, "best-results CSV", |index, record| {
        if index == 1 {
            return read::headers(path, record, &HEADERS);
        }
        let Some(expected) = rows.get(position) else {
            return Err(defect(format!(
                "{} record {index} is an unexpected extra best-results row",
                path.display()
            )));
        };
        read::compare_record(path, index, record, &cells(expected))?;
        position = position.saturating_add(1);
        Ok(())
    })?;
    if total == 0 {
        return Err(defect(format!(
            "{} is empty; the best-results header is missing",
            path.display()
        )));
    }
    let written = total.saturating_sub(1);
    if written == rows.len() {
        Ok(())
    } else {
        Err(defect(format!(
            "{} holds {written} best-results rows where the shared reduction selected {}",
            path.display(),
            rows.len()
        )))
    }
}

fn verify_jsonl(path: &Path, rows: &[selections::SharedSelection]) -> ReportResult<()> {
    let mut position = 0usize;
    let total = read::lines(path, LIMITS, "best-results JSONL", |line, text| {
        let found: serde_json::Value =
            serde_json::from_str(text).map_err(|source| decode(path, line, source))?;
        let Some(expected) = rows.get(position) else {
            return Err(defect(format!(
                "{} line {line} is an unexpected extra best-results record",
                path.display()
            )));
        };
        let expected =
            serde_json::to_value(expected).map_err(|source| decode(path, line, source))?;
        if found == expected {
            position = position.saturating_add(1);
            Ok(())
        } else {
            Err(defect(format!(
                "{} line {line} does not match the shared best-results reduction: {}",
                path.display(),
                diff::difference(&expected, &found)
            )))
        }
    })?;
    if total == rows.len() {
        Ok(())
    } else {
        Err(defect(format!(
            "{} holds {total} best-results records where the shared reduction selected {}",
            path.display(),
            rows.len()
        )))
    }
}

fn cells(row: &selections::SharedSelection) -> Vec<Cell> {
    vec![
        Cell::text(row.athlete_id().as_str()),
        Cell::text(row.athlete.as_str()),
        Cell::optional(row.school.as_deref()),
        Cell::text(row.athlete_state.code()),
        Cell::integer(row.grad_year.to_string()),
        Cell::text(row.gender.stable_key()),
        Cell::text(row.sport()),
        Cell::text(row.key.event_kind.stable_key().into_owned()),
        Cell::text(row.key.surface.label()),
        Cell::text(row.key.wind_class.label()),
        Cell::text(row.key.timing.label()),
        Cell::text(row.mark_text()),
        Cell::integer(row.value.to_string()),
        Cell::text(row.measure().as_str()),
        Cell::text(row.date.as_str()),
        Cell::text(row.meet.as_str()),
        Cell::optional_integer(row.place.map(|place| place.to_string())),
        Cell::decimal(row.wind_mps),
        Cell::optional(row.timing.map(|timing| timing.stable_key())),
        Cell::integer(row.population.marks.to_string()),
        Cell::optional(row.profile_url.as_deref()),
        Cell::text(row.performance_id.as_str()),
        Cell::text(row.source_athlete.as_str()),
        Cell::text(row.source_key.as_str()),
    ]
}

fn decode(path: &Path, line: usize, source: serde_json::Error) -> ReportError {
    ReportError::Decode {
        path: path.to_path_buf(),
        line,
        source,
    }
}
