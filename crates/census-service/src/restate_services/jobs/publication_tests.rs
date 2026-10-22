use super::workbook_job::build;
use super::*;
use census_domain::model::CanonicalSchool;
use census_report::export::{decoded_generations, peak_resident_generations, resident_generations};
use census_report::workbook;
use std::path::Path;

use super::selected_fixture;
mod readback;
mod selected_offline_cli;
mod selected_options;
mod selected_request_admission;
pub(in crate::restate_services) use readback::check_selected_output;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod report_split_fixture;

fn seed(store: &Store, name: &str) -> TestResult {
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name, None);
    store.append(Table::Schools, &school)?;
    Ok(())
}

#[test]
fn managed_workbook_job_decodes_one_dataset_for_every_revision_eight_artifact() -> TestResult {
    if !isolated("managed_workbook_job_decodes_one_dataset_for_every_revision_eight_artifact")? {
        return Ok(());
    }
    let controlled = selected_fixture::install()?;
    let selection = controlled.selection()?;
    let request = census_report::export::ExportJobRequest::SelectedPublicationV1 { selection };
    let store = controlled.store();
    let decoded = decoded_generations();
    let resident = resident_generations();
    let reply = build(
        store,
        &selected_options(controlled.profile())?,
        "managed-publication-fixture",
        &request,
    )?;
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("counter overflow")?);
    check!(eq; resident_generations(), resident);
    check!(eq; peak_resident_generations(), 1);
    let generation = Path::new(&reply.path).parent().ok_or("generation absent")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision"), Some(&serde_json::json!(11)));
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_object)
        .ok_or("artifacts absent")?;
    check!(eq; artifacts.len(), 24);
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
        "current_affiliations.csv",
        "participation_exclusions.csv",
    ] {
        check!(artifacts.contains_key(name), "missing {name}");
    }
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized row counts absent")?;
    require_selected_csv_families(counts)?;
    check!(eq; counts.get("combined_exclusions.csv"), Some(&serde_json::json!(0)));
    let verified = workbook::publication::verify_published(Path::new(&reply.path))?;
    check!(eq; manifest.get("generation_digest").and_then(serde_json::Value::as_str), Some(verified.generation_digest.as_str()));
    Ok(())
}

fn require_selected_csv_families(
    counts: &serde_json::Map<String, serde_json::Value>,
) -> TestResult {
    check!(eq;
        counts.keys().map(String::as_str).collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "athletes.csv",
            "best_marks.csv",
            "coach_contacts.csv",
            "combined_exclusions.csv",
            "comparison_classes.csv",
            "coverage.csv",
            "current_affiliations.csv",
            "identity_aliases.csv",
            "participation_exclusions.csv",
            "provenance.csv",
            "schools.csv",
        ])
    );
    Ok(())
}

#[test]
fn managed_workbook_job_refuses_changed_evidence_without_replacing_its_generation() -> TestResult {
    if !isolated("managed_workbook_job_refuses_changed_evidence_without_replacing_its_generation")?
    {
        return Ok(());
    }
    let controlled = selected_fixture::install()?;
    let selection = controlled.selection()?;
    let job = census_report::export::ExportJobRequest::SelectedPublicationV1 { selection };
    let store = controlled.store();
    let options = selected_options(controlled.profile())?;
    let reply = build(store, &options, "managed-retry-fixture", &job)?;
    let workbook_bytes = std::fs::read(&reply.path)?;
    let generation = Path::new(&reply.path).parent().ok_or("generation absent")?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let repeated = build(store, &options, "managed-retry-fixture", &job)?;
    check!(eq; repeated.path, reply.path);
    seed(store, "Later High School")?;
    let decoded = decoded_generations();
    let error = build(store, &options, "managed-retry-fixture", &job)
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
    let first = build(controlled.store(), &options, job, &export_job)?;
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
    let controlled = selected_fixture::reopen(controlled)?;
    let repeated = build(controlled.store(), &options, job, &export_job)?;
    check!(eq; repeated.path, first.path);
    check!(eq;
        census_report::workbook::publication::verify_published(Path::new(&repeated.path))?.generation_digest,
        generation.generation_digest
    );

    selected_fixture::revoke(&controlled)?;
    check!(build(controlled.store(), &options, job, &export_job).is_err());
    check!(eq; std::fs::read(&first.path)?, bytes);
    check!(eq; std::fs::read(&manifest_path)?, manifest);
    readback::verify_fresh_process(
        NAME,
        &first.path,
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
    )?;
    Ok(())
}

#[tokio::test]
async fn selected_workbook_service_after_split_reopens_successors_and_preserves_prior_generation(
) -> TestResult {
    const NAME: &str =
        "selected_workbook_service_after_split_reopens_successors_and_preserves_prior_generation";
    if !isolated(NAME)? {
        return Ok(());
    }
    report_split_fixture::service_split_acceptance().await
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
