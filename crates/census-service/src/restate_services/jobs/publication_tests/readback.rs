use super::{Path, TestResult};

pub(in crate::restate_services) fn check_selected_output(
    path: &Path,
    public: &str,
    candidate: &str,
    school: &str,
    mark: &str,
) -> TestResult {
    let generation = path
        .parent()
        .ok_or("selected workbook generation missing")?;
    check_csv_row_fields(
        &generation.join("athletes.csv"),
        "athlete_id",
        public,
        &[
            ("performance_availability", "eligible_selected_mark"),
            ("identity_status", "accepted_public_identity"),
        ],
    )?;
    check_csv_row_fields(
        &generation.join("best_marks.csv"),
        "athlete_id",
        public,
        &[("display_mark", mark)],
    )?;
    check_csv_row_fields(
        &generation.join("current_affiliations.csv"),
        "retained_candidate_id",
        candidate,
        &[
            ("current_school_id", school),
            ("resolution_status", "resolved_at_selected_cutoff"),
        ],
    )
}
fn check_csv_row_fields(
    path: &Path,
    key_field: &str,
    key: &str,
    fields: &[(&str, &str)],
) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let key_index = headers
        .iter()
        .position(|header| header == key_field)
        .ok_or_else(|| format!("field {key_field} missing in {}", path.display()))?;
    let mut matched = None;
    for row in reader.records() {
        let row = row?;
        if row.get(key_index) == Some(key) {
            if matched.is_some() {
                return Err(format!(
                    "multiple rows with {key_field}={key:?} in {}",
                    path.display()
                )
                .into());
            }
            matched = Some(row);
        }
    }
    let row = matched
        .ok_or_else(|| format!("row with {key_field}={key:?} missing in {}", path.display()))?;
    for &(field, expected) in fields {
        let index = headers
            .iter()
            .position(|header| header == field)
            .ok_or_else(|| format!("field {field} missing in {}", path.display()))?;
        let actual = row.get(index).ok_or("matched CSV row field absent")?;
        if actual != expected {
            return Err(format!(
                "{} field {field}: expected {expected:?}, got {actual:?}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn verify_fresh_process(
    name: &str,
    path: &str,
    public: &str,
    candidate: &str,
    school: &str,
) -> TestResult {
    let full_name = format!("restate_services::jobs::publication_tests::{name}");
    let child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", &full_name, "--test-threads=1", "--nocapture"])
        .env("CENSUS_PUBLICATION_COUNTER_TEST", &full_name)
        .env("CENSUS_SELECTED_WORKBOOK_VERIFY_PATH", path)
        .env("CENSUS_SELECTED_WORKBOOK_VERIFY_PUBLIC_ID", public)
        .env("CENSUS_SELECTED_WORKBOOK_VERIFY_CANDIDATE_ID", candidate)
        .env("CENSUS_SELECTED_WORKBOOK_VERIFY_SCHOOL_ID", school)
        .current_dir(std::env::temp_dir())
        .output()?;
    check!(
        child.status.success(),
        "fresh-process verifier failed: {} {}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    check!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    Ok(())
}
