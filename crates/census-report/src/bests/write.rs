use super::SharedSelection;
use census_store::read::csv_failure;
use census_store::{StoreError, StoreResult};
use std::path::{Path, PathBuf};

mod row;
use row::Row;

const HEADER: [&str; 28] = [
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

pub fn write(
    out_dir: &Path,
    rows: &[SharedSelection],
    cohort: &str,
) -> StoreResult<(PathBuf, PathBuf)> {
    census_store::fs::create_dir_all_synced(out_dir).map_err(|source| StoreError::Io {
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
    rows.iter().try_for_each(|row| {
        let csv_row: Row<'_> = row.into();
        writer
            .serialize(&csv_row)
            .map_err(|error| csv_failure(temporary, error))
    })?;
    writer.flush().map_err(|source| StoreError::Io {
        path: temporary.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests;
