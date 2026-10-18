use super::*;
use census_domain::model::CanonicalSchool;
use census_report::export::{decoded_generations, resident_generations};
use std::path::Path;

use super::selected_fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn options() -> TestResult<workbook::Options> {
    Ok(workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
    })
}

fn seed(store: &Store, name: &str) -> TestResult {
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name, None);
    store.append(Table::Schools, &school)?;
    Ok(())
}

#[test]
fn managed_workbook_job_decodes_one_dataset_for_every_revision_six_artifact() -> TestResult {
    if !isolated("managed_workbook_job_decodes_one_dataset_for_every_revision_six_artifact")? {
        return Ok(());
    }
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store, "Original High School")?;
    let decoded = decoded_generations();
    let resident = resident_generations();
    let reply = build_workbook(
        &store,
        &options()?,
        "managed-publication-fixture",
        &census_report::export::ExportJobRequest::LegacyV2,
    )?;
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("counter overflow")?);
    check!(eq; resident_generations(), resident);
    let generation = Path::new(&reply.path).parent().ok_or("generation absent")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision"), Some(&serde_json::json!(6)));
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_object)
        .ok_or("artifacts absent")?;
    check!(eq; artifacts.len(), 22);
    for name in [
        "census-core.json",
        "census-all-sources.json",
        "best-results-co2027.csv",
        "best-results-co2027.jsonl",
        "athletes.csv",
        "best_marks.csv",
        "schools.csv",
        "coach_contacts.csv",
        "comparison_classes.csv",
        "provenance.csv",
        "coverage.csv",
        "identity_aliases.csv",
        "combined_exclusions.csv",
    ] {
        check!(artifacts.contains_key(name), "missing {name}");
    }
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized row counts absent")?;
    check!(eq; counts.len(), 9);
    check!(eq; counts.get("combined_exclusions.csv"), Some(&serde_json::json!(0)));
    let verified = workbook::publication::verify_published(Path::new(&reply.path))?;
    check!(eq; manifest.get("generation_digest").and_then(serde_json::Value::as_str), Some(verified.generation_digest.as_str()));
    Ok(())
}

#[test]
fn managed_workbook_job_refuses_changed_evidence_without_replacing_its_generation() -> TestResult {
    if !isolated("managed_workbook_job_refuses_changed_evidence_without_replacing_its_generation")?
    {
        return Ok(());
    }
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store, "Original High School")?;
    let options = options()?;
    let job = census_report::export::ExportJobRequest::LegacyV2;
    let reply = build_workbook(&store, &options, "managed-retry-fixture", &job)?;
    let workbook_bytes = std::fs::read(&reply.path)?;
    let generation = Path::new(&reply.path).parent().ok_or("generation absent")?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let repeated = build_workbook(&store, &options, "managed-retry-fixture", &job)?;
    check!(eq; repeated.path, reply.path);
    seed(&store, "Later High School")?;
    let decoded = decoded_generations();
    let error = build_workbook(&store, &options, "managed-retry-fixture", &job)
        .err()
        .ok_or("changed source evidence reused the same job")?;
    check!(
        error.to_string().contains("stale or foreign export input"),
        "{error}"
    );
    check!(eq; decoded_generations(), decoded);
    check!(eq; std::fs::read(&reply.path)?, workbook_bytes);
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    Ok(())
}

#[test]
fn selected_workbook_service_job_publishes_and_fresh_process_verifies_retry_and_revocation(
) -> TestResult {
    const NAME: &str =
        "selected_workbook_service_job_publishes_and_fresh_process_verifies_retry_and_revocation";
    if !isolated(NAME)? {
        return Ok(());
    }
    match std::env::var("CENSUS_SELECTED_WORKBOOK_VERIFY_PATH") {
        Ok(path) => {
            verify_selected_workbook(&path)?;
            return Ok(());
        }
        Err(std::env::VarError::NotPresent) => {}
        Err(error) => return Err(error.into()),
    }

    let controlled = selected_fixture::install()?;
    let selection = controlled.selection()?;
    let export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 { selection };
    let options = selected_options(controlled.profile())?;
    let job = "controlled-selected-service-workbook-v1";
    let first = build_workbook(controlled.store(), &options, job, &export_job)?;
    let generation =
        census_report::workbook::publication::verify_published(Path::new(&first.path))?;
    check!(generation.workbook.mapped_athletes > 0);
    check_selected_output(
        Path::new(&first.path),
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        "50.00",
    )?;
    let bytes = std::fs::read(&first.path)?;
    let manifest_path = Path::new(&first.path)
        .parent()
        .ok_or("selected workbook generation missing")?
        .join("manifest.json");
    let manifest = std::fs::read(&manifest_path)?;
    let repeated = build_workbook(controlled.store(), &options, job, &export_job)?;
    check!(eq; repeated.path, first.path);
    check!(eq;
        census_report::workbook::publication::verify_published(Path::new(&repeated.path))?.generation_digest,
        generation.generation_digest
    );

    selected_fixture::revoke(&controlled)?;
    check!(build_workbook(controlled.store(), &options, job, &export_job).is_err());
    check!(eq; std::fs::read(&first.path)?, bytes);
    check!(eq; std::fs::read(&manifest_path)?, manifest);
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            &format!("restate_services::jobs::publication_tests::{NAME}"),
            "--test-threads=1",
            "--nocapture",
        ])
        .env(
            "CENSUS_PUBLICATION_COUNTER_TEST",
            format!("restate_services::jobs::publication_tests::{NAME}"),
        )
        .env("CENSUS_SELECTED_WORKBOOK_VERIFY_PATH", &first.path)
        .env(
            "CENSUS_SELECTED_WORKBOOK_VERIFY_PUBLIC_ID",
            controlled.public_id(),
        )
        .env(
            "CENSUS_SELECTED_WORKBOOK_VERIFY_CANDIDATE_ID",
            controlled.candidate_id(),
        )
        .env(
            "CENSUS_SELECTED_WORKBOOK_VERIFY_SCHOOL_ID",
            controlled.current_school_id(),
        )
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

pub(in crate::restate_services) fn selected_options(
    profile: &census_domain::model::QueryProfileV1,
) -> TestResult<workbook::Options> {
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("controlled selection must contain one cohort".into());
    };
    Ok(workbook::Options {
        grad_year: Some(cohort.get()),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: profile.input().jurisdictions.clone(),
        school_year: profile.input().contact_school_year,
    })
}

fn verify_selected_workbook(path: &str) -> TestResult {
    let verified = census_report::workbook::publication::verify_published(Path::new(path))?;
    check!(verified.workbook.mapped_athletes > 0);
    let public = std::env::var("CENSUS_SELECTED_WORKBOOK_VERIFY_PUBLIC_ID")?;
    let candidate = std::env::var("CENSUS_SELECTED_WORKBOOK_VERIFY_CANDIDATE_ID")?;
    let school = std::env::var("CENSUS_SELECTED_WORKBOOK_VERIFY_SCHOOL_ID")?;
    check_selected_output(Path::new(path), &public, &candidate, &school, "50.00")?;
    Ok(())
}

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
    check_csv_row_field(
        &generation.join("athletes.csv"),
        "athlete_id",
        public,
        "athlete_id",
        public,
    )?;
    check_csv_row_field(
        &generation.join("athletes.csv"),
        "athlete_id",
        public,
        "performance_availability",
        "eligible_selected_mark",
    )?;
    check_csv_row_field(
        &generation.join("athletes.csv"),
        "athlete_id",
        public,
        "identity_status",
        "accepted_public_identity",
    )?;
    check_csv_row_field(
        &generation.join("best_marks.csv"),
        "athlete_id",
        public,
        "display_mark",
        mark,
    )?;
    check_csv_row_field(
        &generation.join("current_affiliations.csv"),
        "retained_candidate_id",
        candidate,
        "current_school_id",
        school,
    )?;
    check_csv_row_field(
        &generation.join("current_affiliations.csv"),
        "retained_candidate_id",
        candidate,
        "resolution_status",
        "resolved_at_selected_cutoff",
    )?;
    check_csv_row_field(
        &generation.join("best-results-co2027.csv"),
        "athlete_id",
        public,
        "athlete_id",
        public,
    )?;
    Ok(())
}

fn check_csv_row_field(
    path: &Path,
    key_field: &str,
    key: &str,
    field: &str,
    expected: &str,
) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let key_index = headers
        .iter()
        .position(|header| header == key_field)
        .ok_or_else(|| format!("field {key_field} missing in {}", path.display()))?;
    let value_index = headers
        .iter()
        .position(|header| header == field)
        .ok_or_else(|| format!("field {field} missing in {}", path.display()))?;
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
    let actual = row
        .get(value_index)
        .ok_or_else(|| format!("field {field} missing in matched row of {}", path.display()))?;
    if actual != expected {
        return Err(format!(
            "{} field {field}: expected {expected:?}, got {actual:?}",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn isolated(name: &str) -> TestResult<bool> {
    let full_name = format!("restate_services::jobs::publication_tests::{name}");
    if std::env::var("CENSUS_PUBLICATION_COUNTER_TEST").as_deref() == Ok(full_name.as_str()) {
        return Ok(true);
    }
    let directory = tempfile::tempdir()?;
    let result = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", &full_name, "--test-threads=1", "--nocapture"])
        .env("CENSUS_PUBLICATION_COUNTER_TEST", &full_name)
        .current_dir(directory.path())
        .output()?;
    check!(
        result.status.success(),
        "child failed: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    check!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    Ok(false)
}
