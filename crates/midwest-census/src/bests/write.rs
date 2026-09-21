//! The sidecars beside the workbook: one JSONL row per best mark, and its CSV twin.

use super::BestResult;
use crate::store::{Store, StoreError, StoreResult};
use std::io::{BufWriter, Write};
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
    write_jsonl(&jsonl, rows)?;
    write_csv(&csv_path, rows)?;
    Ok((jsonl, csv_path))
}

/// One JSON object per row, newline-terminated.
fn write_jsonl(path: &Path, rows: &[BestResult]) -> StoreResult<()> {
    let file = std::fs::File::create(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut json = BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut json, row).map_err(|source| StoreError::Json {
            detail: format!("writing {}", path.display()),
            source,
        })?;
        json.write_all(b"\n").map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    json.flush().map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// The CSV twin: the fixed header, then one serialized record per row.
fn write_csv(path: &Path, rows: &[BestResult]) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)
        .map_err(|error| csv_failure(path, error))?;
    writer
        .write_record(HEADER)
        .map_err(|error| csv_failure(path, error))?;
    for row in rows {
        writer
            .serialize(row)
            .map_err(|error| csv_failure(path, error))?;
    }
    writer.flush().map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// A `csv` failure: a file the sidecar writes to keeps its path and source, and a row the writer
/// refused is reported with the message the writer raised.
fn csv_failure(path: &Path, error: csv::Error) -> StoreError {
    let detail = error.to_string();
    match error.into_kind() {
        csv::ErrorKind::Io(source) => StoreError::Io {
            path: path.to_path_buf(),
            source,
        },
        _ => StoreError::Invariant {
            detail: format!("writing {}: {detail}", path.display()),
        },
    }
}
