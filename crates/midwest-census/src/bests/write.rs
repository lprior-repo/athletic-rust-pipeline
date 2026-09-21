//! The sidecars beside the workbook: one JSONL row per best mark, and its CSV twin.

use super::BestResult;
use crate::store::Store;
use anyhow::Result;
use std::path::PathBuf;

/// Write the reduction as `out/best-results-<cohort>.jsonl` and `.csv`.
pub fn write(store: &Store, rows: &[BestResult], cohort: &str) -> Result<(PathBuf, PathBuf)> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let jsonl = out.join(format!("best-results-{cohort}.jsonl"));
    let csv_path = out.join(format!("best-results-{cohort}.csv"));

    let mut json = std::io::BufWriter::new(std::fs::File::create(&jsonl)?);
    for row in rows {
        serde_json::to_writer(&mut json, row)?;
        std::io::Write::write_all(&mut json, b"\n")?;
    }
    std::io::Write::flush(&mut json)?;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&csv_path)?;
    writer.write_record([
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
    ])?;
    for row in rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok((jsonl, csv_path))
}
