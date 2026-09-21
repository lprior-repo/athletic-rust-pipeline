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
    let mut csv = String::from(
        "state,schools,athletes,co2027,co2027_boys,co2027_girls,co2027_profile_url,co2027_grade_evidence,co2027_multisource,co2027_with_coach,co2027_with_coach_email,coaches,coaches_with_email\n",
    );
    let mut rows: Vec<&StateCensus> = census.by_state.values().collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.class_of_2027));
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            row.state,
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
        ));
    }
    csv.push_str(&format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        census.totals.state,
        census.totals.schools,
        census.totals.athletes,
        census.totals.class_of_2027,
        census.totals.class_of_2027_boys,
        census.totals.class_of_2027_girls,
        census.totals.class_of_2027_with_profile_url,
        census.totals.class_of_2027_with_grad_year_evidence,
        census.totals.class_of_2027_multisource,
        census.totals.class_of_2027_with_coach,
        census.totals.class_of_2027_with_coach_email,
        census.totals.coaches,
        census.totals.coaches_with_email
    ));
    std::fs::write(&csv_path, csv).map_err(|source| io_error(&csv_path, source))?;
    Ok((json_path, csv_path))
}
