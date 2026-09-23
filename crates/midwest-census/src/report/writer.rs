//! Publication: `report*.json` and the flat `census-by-state*.csv` next to the logs.

use super::{io_error, Census, ReportError, ReportResult, Scope, StateCensus};
use census_store::read::publish_atomically;
use census_store::{Store, StoreError, StoreResult};

/// Write the census JSON and a flat per-state CSV next to the consolidated logs.
///
/// Both are published by rename: the body is staged in a temporary beside the destination and the
/// rename is the publication, so a reader that opens either name sees the artifact this run wrote or
/// the one before it, never the truncated file an interrupted in-place write leaves.
pub fn write_census(
    store: &Store,
    census: &Census,
    scope: Scope,
) -> ReportResult<(std::path::PathBuf, std::path::PathBuf)> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out).map_err(|source| io_error(&out, source))?;
    let json_path = out.join(format!("report{}.json", scope.file_suffix()));
    let json = serde_json::to_vec_pretty(census).map_err(|source| ReportError::Invariant {
        detail: format!("the census is not valid json: {source}"),
    })?;
    publish_atomically(&json_path, |temporary| {
        write_body(temporary, &json_path, &json)
    })?;
    let csv_path = out.join(format!("census-by-state{}.csv", scope.file_suffix()));
    let csv = census_csv(census);
    publish_atomically(&csv_path, |temporary| {
        write_body(temporary, &csv_path, csv.as_bytes())
    })?;
    Ok((json_path, csv_path))
}

/// Write `bytes` into `temporary`, the file [`publish_atomically`] renames to `published`: the body
/// stages beside the destination, so the rename is the publication and a refused body leaves the
/// published artifact exactly as it was.
fn write_body(
    temporary: &std::path::Path,
    published: &std::path::Path,
    bytes: &[u8],
) -> StoreResult<()> {
    std::fs::write(temporary, bytes).map_err(|source| StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}

/// The flat per-state CSV: the header, one line per jurisdiction, and the `TOTAL` line.
fn census_csv(census: &Census) -> String {
    let mut csv = String::from(
        "state,schools,athletes,co2027,co2027_boys,co2027_girls,co2027_profile_url,co2027_grade_evidence,co2027_multisource,co2027_with_coach,co2027_with_coach_email,coaches,coaches_with_email\n",
    );
    let mut rows: Vec<&StateCensus> = census.by_state.values().collect();
    // Most athletes first; ties keep the printed label ascending, which is the row order this CSV
    // published while the bucket was still a string key — and a reordered published file is a
    // different file.
    rows.sort_by(|left, right| {
        right
            .class_of_2027
            .cmp(&left.class_of_2027)
            .then_with(|| left.state.code().cmp(right.state.code()))
    });
    for row in rows {
        csv.push_str(&csv_row(&row.state.to_string(), row));
    }
    csv.push_str(&csv_row("TOTAL", &census.totals));
    csv
}

/// One CSV line: the printed label, then the row's thirteen counters in header order.
fn csv_row(label: &str, row: &StateCensus) -> String {
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        label,
        row.schools,
        row.athletes,
        row.class_of_2027,
        row.class_of_2027_boys,
        row.class_of_2027_girls,
        row.class_of_2027_with_profile_url,
        row.class_of_2027_with_grad_year_evidence,
        row.class_of_2027_multisource,
        row.class_of_2027_with_coach,
        row.class_of_2027_with_coach_email,
        row.coaches,
        row.coaches_with_email
    )
}
