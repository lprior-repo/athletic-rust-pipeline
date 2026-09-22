//! Publication: `report*.json` and the flat `census-by-state*.csv` next to the logs.

use super::{io_error, Census, ReportError, ReportResult, Scope, StateCensus};
use crate::store::Store;

/// Write the census JSON and a flat per-state CSV next to the consolidated logs.
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
    std::fs::write(&json_path, json).map_err(|source| io_error(&json_path, source))?;
    let csv_path = out.join(format!("census-by-state{}.csv", scope.file_suffix()));
    let csv = census_csv(census);
    std::fs::write(&csv_path, csv).map_err(|source| io_error(&csv_path, source))?;
    Ok((json_path, csv_path))
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
