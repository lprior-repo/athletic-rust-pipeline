//! The sidecars beside the workbook: one JSONL row per best mark, and its CSV twin.

use super::BestResult;
use census_store::read::csv_failure;
use census_store::{Store, StoreError, StoreResult};
use std::path::{Path, PathBuf};

/// The CSV header, one column per field [`BestResult`] serializes.
const HEADER: [&str; 18] = [
    "athlete_id",
    "name",
    "school",
    "state",
    "grad_year",
    "gender",
    "sport",
    "event",
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
];

/// Write the reduction as `out/best-results-<cohort>.jsonl` and `.csv`.
pub fn write(store: &Store, rows: &[BestResult], cohort: &str) -> StoreResult<(PathBuf, PathBuf)> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out).map_err(|source| StoreError::Io {
        path: out.clone(),
        source,
    })?;
    let jsonl = out.join(format!("best-results-{cohort}.jsonl"));
    let csv_path = out.join(format!("best-results-{cohort}.csv"));
    census_store::read::write_snapshot_rows(&jsonl, rows)?;
    write_csv(&csv_path, rows)?;
    Ok((jsonl, csv_path))
}

/// The CSV twin: the fixed header, then one serialized record per row.
///
/// Published exactly like the JSONL half — a temporary beside the destination, renamed over the name
/// only once its bytes are on the disk — because the pair is one artifact: a reader that finds the CSV
/// shorter than the JSONL beside it has no way to tell a torn write from a real reduction.
fn write_csv(path: &Path, rows: &[BestResult]) -> StoreResult<()> {
    census_store::read::publish_atomically(path, |temporary| write_csv_body(temporary, path, rows))
}

/// Encode the CSV twin into `temporary`, the file the publication renames to `published`.
fn write_csv_body(temporary: &Path, published: &Path, rows: &[BestResult]) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(temporary)
        .map_err(|error| csv_failure(published, error))?;
    writer
        .write_record(HEADER)
        .map_err(|error| csv_failure(published, error))?;
    for row in rows {
        writer
            .serialize(row)
            .map_err(|error| csv_failure(published, error))?;
    }
    writer.flush().map_err(|source| StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}
