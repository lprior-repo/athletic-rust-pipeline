use super::SharedSelection;
use census_store::read::csv_failure;
use census_store::{StoreError, StoreResult};
use std::path::{Path, PathBuf};

const HEADER: [&str; 24] = [
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

#[derive(serde::Serialize)]
pub(crate) struct Row<'a> {
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    athlete_id: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    name: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_optional_text")]
    school: Option<&'a str>,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    state: &'static str,
    grad_year: i16,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    gender: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    sport: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    event: std::borrow::Cow<'a, str>,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    surface: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    wind_class: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    timing_class: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    best_mark: String,
    best_value: i64,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    measure: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    date: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    meet: &'a str,
    place: Option<u16>,
    wind_mps: Option<f64>,
    #[serde(serialize_with = "crate::csv_safety::serialize_optional_text")]
    timing: Option<&'a str>,
    marks_in_event: usize,
    #[serde(serialize_with = "crate::csv_safety::serialize_optional_text")]
    profile_url: Option<&'a str>,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    performance_id: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    source_athlete: &'a str,
    #[serde(serialize_with = "crate::csv_safety::serialize_text")]
    source_key: &'a str,
}

impl<'a> From<&'a SharedSelection> for Row<'a> {
    fn from(row: &'a SharedSelection) -> Self {
        Row {
            athlete_id: row.key.athlete_id.as_str(),
            name: &row.athlete,
            school: row.school.as_deref(),
            state: row.athlete_state.code(),
            grad_year: row.grad_year,
            gender: row.gender.stable_key(),
            sport: row.sport(),
            event: row.key.event_kind.stable_key(),
            surface: row.key.surface.label(),
            wind_class: row.key.wind_class.label(),
            timing_class: row.key.timing.label(),
            best_mark: row.mark_text(),
            best_value: row.value,
            measure: row.measure().as_str(),
            date: &row.date,
            meet: &row.meet,
            place: row.place,
            wind_mps: row.wind_mps,
            timing: row.timing.map(|t| t.stable_key()),
            marks_in_event: row.population.marks,
            profile_url: row.profile_url.as_deref(),
            performance_id: row.performance_id.as_str(),
            source_athlete: &row.source_athlete,
            source_key: &row.source_key,
        }
    }
}

pub fn write(
    out_dir: &Path,
    rows: &[SharedSelection],
    cohort: &str,
) -> StoreResult<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(out_dir).map_err(|source| StoreError::Io {
        path: out_dir.to_path_buf(),
        source,
    })?;
    let jsonl = out_dir.join(format!("best-results-{cohort}.jsonl"));
    let csv_path = out_dir.join(format!("best-results-{cohort}.csv"));
    census_store::read::write_snapshot_rows(&jsonl, rows)?;
    write_csv(&csv_path, rows)?;
    Ok((jsonl, csv_path))
}

fn write_csv(path: &Path, rows: &[SharedSelection]) -> StoreResult<()> {
    census_store::read::publish_atomically(path, |temporary| write_csv_body(temporary, rows))
}

fn write_csv_body(temporary: &Path, rows: &[SharedSelection]) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(temporary)
        .map_err(|error| csv_failure(temporary, error))?;
    writer
        .write_record(HEADER)
        .map_err(|error| csv_failure(temporary, error))?;
    for row in rows {
        let csv_row: Row = row.into();
        writer
            .serialize(&csv_row)
            .map_err(|error| csv_failure(temporary, error))?;
    }
    writer.flush().map_err(|source| StoreError::Io {
        path: temporary.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests;
