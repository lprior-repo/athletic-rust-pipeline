use super::workbook_job::build;
use super::*;
use census_domain::model::CanonicalSchool;
use census_report::export::{decoded_generations, resident_generations};
use census_report::workbook;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
    let reply = build(
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
    let reply = build(&store, &options, "managed-retry-fixture", &job)?;
    let workbook_bytes = std::fs::read(&reply.path)?;
    let generation = Path::new(&reply.path).parent().ok_or("generation absent")?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let repeated = build(&store, &options, "managed-retry-fixture", &job)?;
    check!(eq; repeated.path, reply.path);
    seed(&store, "Later High School")?;
    let decoded = decoded_generations();
    let error = build(&store, &options, "managed-retry-fixture", &job)
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
