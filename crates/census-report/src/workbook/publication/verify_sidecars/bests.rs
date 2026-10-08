use std::path::Path;

use crate::bests as selections;
use crate::report::{ReportError, ReportResult};

use super::cells::Cell;
use super::input::Inputs;
use super::read::{self, Limits};
use super::{cohort, defect, diff};

const HEADERS: [&str; 28] = [
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
    "best_value_unit",
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
    "comparison_policy",
    "comparison_specification",
    "source_specification",
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
        verify_csv_row(path, index, record, rows, &mut position)
    })?;
    verify_csv_count(path, total, rows.len())
}

fn verify_csv_count(path: &Path, total: usize, expected: usize) -> ReportResult<()> {
    if total == 0 {
        return Err(defect(format!(
            "{} is empty; the best-results header is missing",
            path.display()
        )));
    }
    let written = total.saturating_sub(1);
    if written == expected {
        Ok(())
    } else {
        Err(defect(format!(
            "{} holds {written} best-results rows where the shared reduction selected {}",
            path.display(),
            expected
        )))
    }
}

fn verify_csv_row(
    path: &Path,
    index: usize,
    record: &csv::StringRecord,
    rows: &[selections::SharedSelection],
    position: &mut usize,
) -> ReportResult<()> {
    if index == 1 {
        return read::headers(path, record, &HEADERS);
    }
    let Some(expected) = rows.get(*position) else {
        return Err(defect(format!(
            "{} record {index} is an unexpected extra best-results row",
            path.display()
        )));
    };
    read::compare_record(path, index, record, &cells(path, expected)?)?;
    *position = position.saturating_add(1);
    Ok(())
}

fn verify_jsonl(path: &Path, rows: &[selections::SharedSelection]) -> ReportResult<()> {
    let mut position = 0usize;
    let total = read::lines(path, LIMITS, "best-results JSONL", |line, text| {
        verify_jsonl_row(path, line, text, rows, &mut position)
    })?;
    verify_jsonl_count(path, total, rows.len())
}

fn verify_jsonl_count(path: &Path, total: usize, expected: usize) -> ReportResult<()> {
    if total == expected {
        Ok(())
    } else {
        Err(defect(format!(
            "{} holds {total} best-results records where the shared reduction selected {}",
            path.display(),
            expected
        )))
    }
}

fn verify_jsonl_row(
    path: &Path,
    line: usize,
    text: &str,
    rows: &[selections::SharedSelection],
    position: &mut usize,
) -> ReportResult<()> {
    let found: serde_json::Value =
        serde_json::from_str(text).map_err(|source| decode(path, line, source))?;
    let expected = rows.get(*position).ok_or_else(|| {
        defect(format!(
            "{} line {line} is an unexpected extra best-results record",
            path.display()
        ))
    })?;
    let expected = serde_json::to_value(expected).map_err(|source| decode(path, line, source))?;
    compare_jsonl(path, line, &expected, &found)?;
    *position = position.saturating_add(1);
    Ok(())
}

fn compare_jsonl(
    path: &Path,
    line: usize,
    expected: &serde_json::Value,
    found: &serde_json::Value,
) -> ReportResult<()> {
    if found == expected {
        Ok(())
    } else {
        Err(defect(format!(
            "{} line {line} does not match the shared best-results reduction: {}",
            path.display(),
            diff::difference(expected, found)
        )))
    }
}

fn cells(path: &Path, row: &selections::SharedSelection) -> ReportResult<Vec<Cell>> {
    Ok(identity_cells(row)
        .into_iter()
        .chain(result_cells(row))
        .chain(specification_cells(path, row)?)
        .collect())
}

fn specification_cells(path: &Path, row: &selections::SharedSelection) -> ReportResult<[Cell; 3]> {
    let policy = match row.key.comparison {
        selections::ComparisonPolicy::Standard => "standard",
        selections::ComparisonPolicy::ObservedFastest => "observed_fastest",
        selections::ComparisonPolicy::SameCourse => "same_course",
    };
    let comparison =
        serde_json::to_string(&row.key.specification).map_err(|source| decode(path, 0, source))?;
    let source = serde_json::to_string(&row.source.specification)
        .map_err(|source| decode(path, 0, source))?;
    Ok([
        Cell::text(policy),
        Cell::text(comparison),
        Cell::text(source),
    ])
}

fn identity_cells(row: &selections::SharedSelection) -> [Cell; 11] {
    [
        Cell::text(row.athlete_id().as_str()),
        Cell::text(row.athlete.name.as_str()),
        Cell::optional(row.athlete.school.as_deref()),
        Cell::text(row.athlete.athlete_state.code()),
        Cell::integer(row.athlete.grad_year.to_string()),
        Cell::text(row.athlete.gender.stable_key()),
        Cell::text(row.sport()),
        Cell::text(row.key.event_kind.stable_key().into_owned()),
        Cell::text(row.key.surface.label()),
        Cell::text(row.key.wind_class.label()),
        Cell::text(row.key.timing.label()),
    ]
}

fn result_cells(row: &selections::SharedSelection) -> [Cell; 14] {
    [
        Cell::text(row.mark_text()),
        Cell::integer(row.result.value.to_string()),
        Cell::text(row.measure().value_unit()),
        Cell::text(row.measure().as_str()),
        Cell::text(row.meet.date.as_str()),
        Cell::text(row.meet.name.as_str()),
        Cell::optional_integer(row.result.place.map(|place| place.to_string())),
        Cell::decimal(row.result.wind_mps),
        Cell::optional(row.result.timing.map(|timing| timing.stable_key())),
        Cell::integer(row.population.marks.to_string()),
        Cell::optional(row.athlete.profile_url.as_deref()),
        Cell::text(row.source.performance_id.as_str()),
        Cell::text(row.source.source_athlete.as_str()),
        Cell::text(row.source.source_key.as_str()),
    ]
}

fn decode(path: &Path, line: usize, source: serde_json::Error) -> ReportError {
    ReportError::Decode {
        path: path.to_path_buf(),
        line,
        source,
    }
}
