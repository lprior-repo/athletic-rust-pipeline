use super::{io_error, Census, ReportError, ReportResult, Scope, StateCensus};
use census_store::read::publish_atomically;
use census_store::{Store, StoreError, StoreResult};

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

fn census_csv(census: &Census) -> String {
    let mut csv = String::from(
        "state,schools,athletes,co2027,co2027_boys,co2027_girls,co2027_profile_url,co2027_grade_evidence,co2027_multisource,co2027_with_coach,co2027_with_coach_email,coaches,coaches_with_email\n",
    );
    let mut rows: Vec<&StateCensus> = census.by_state.values().collect();
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
