#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "selected_publication_cli/fixture.rs"]
mod fixture;
#[path = "selected_publication_cli/paths.rs"]
mod paths;
#[path = "selected_publication_cli/refusals.rs"]
mod refusals;

use census_report::workbook::publication;
use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn run(root: &Path, selection: &Path, out: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(root)
        .arg("selected-publication")
        .arg("--selection")
        .arg(selection)
        .arg("--out")
        .arg(out)
        .output()?)
}

#[tokio::test]
async fn selected_publication_executable_withholds_unwindowed_marks_and_replays_after_source_deletion(
) -> TestResult {
    let fixture = fixture::Fixture::create().await?;
    let out = fixture.directory.path().join("publication");
    let before = fixture.evidence_state()?;
    let result = run(&fixture.root, &fixture.selection, &out)?;
    check!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let reply: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    check!(eq; reply["qualification_only"], true);
    check!(eq; reply["qualification_lifecycle"], 7);
    check!(eq; reply["schema"], "selected_publication_qualification_v3");
    check!(eq; reply["participation_policy_revision"], 4);
    check!(eq; reply["independently_verified"], true);
    let path = std::path::PathBuf::from(reply["workbook"].as_str().ok_or("workbook path")?);
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    check!(eq; reply["generation_digest"], verified.generation_digest);
    check!(eq; fixture.evidence_state()?, before);
    artifacts(&path, &fixture)?;
    let unchanged = run(&fixture.root, &fixture.selection, &out)?;
    check!(!unchanged.status.success());
    check!(String::from_utf8_lossy(&unchanged.stderr).contains("output already exists"));
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    std::fs::remove_dir_all(&fixture.root)?;
    let offline = publication::verify_published(&path)?;
    check!(eq; offline.generation_digest, verified.generation_digest);
    artifacts(&path, &fixture)?;
    Ok(())
}

#[tokio::test]
async fn current_roster_profile_withholds_unreviewed_source_and_reopens_its_audit() -> TestResult {
    let fixture = fixture::Fixture::create().await?;
    let selection = fixture.select_current_roster_policy()?;
    let before = fixture.evidence_state()?;
    let result = run(
        &fixture.root,
        &selection,
        &fixture.directory.path().join("current-publication"),
    )?;
    check!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let reply: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    check!(eq; reply["qualification_lifecycle"], 8);
    let path = std::path::PathBuf::from(reply["workbook"].as_str().ok_or("workbook path")?);
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    check!(eq; fixture.evidence_state()?, before);
    let directory = path.parent().ok_or("generation directory")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    let digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("frozen input digest")?;
    let frozen = directory.join("frozen-input.json");
    let dataset = census_report::export::ExportDataset::reopen_frozen_with_digest(&frozen, digest)?;
    let audit = unknown_affiliation(directory, &fixture.athlete, &fixture.roster_reference)?;
    check!(dataset
        .performances
        .iter()
        .any(|row| row.id.as_str() == fixture.performance));
    drop(dataset);
    std::fs::remove_dir_all(&fixture.root)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    let reopened =
        census_report::export::ExportDataset::reopen_frozen_with_digest(&frozen, digest)?;
    check!(reopened
        .performances
        .iter()
        .any(|row| row.id.as_str() == fixture.performance));
    check!(eq; unknown_affiliation(directory, &fixture.athlete, &fixture.roster_reference)?, audit);
    Ok(())
}

fn unknown_affiliation(
    directory: &Path,
    athlete: &str,
    source_reference: &str,
) -> TestResult<csv::StringRecord> {
    let mut reader = csv::Reader::from_path(directory.join("current_affiliations.csv"))?;
    let headers = reader.headers()?;
    let candidate = headers
        .iter()
        .position(|name| name == "retained_candidate_id")
        .ok_or("affiliation candidate column")?;
    let school = headers
        .iter()
        .position(|name| name == "current_school_id")
        .ok_or("current school column")?;
    let status = headers
        .iter()
        .position(|name| name == "resolution_status")
        .ok_or("affiliation status column")?;
    let evidence = headers
        .iter()
        .position(|name| name == "evidence_refs")
        .ok_or("affiliation evidence column")?;
    let reasons = headers
        .iter()
        .position(|name| name == "unavailable_reasons")
        .ok_or("affiliation reason column")?;
    for row in reader.records() {
        let row = row?;
        if row.get(candidate) == Some(athlete) {
            check!(eq; row.get(school), Some(""));
            check!(eq; row.get(status), Some("current_affiliation_unknown"));
            let expected_refs = serde_json::to_string(&[source_reference])?;
            check!(eq; row.get(evidence), Some(expected_refs.as_str()));
            check!(eq; row.get(reasons), Some("[\"archive_unavailable\"]"));
            return Ok(row);
        }
    }
    Err("retained candidate absent from affiliation audit".into())
}

fn artifacts(path: &Path, expected: &fixture::Fixture) -> TestResult {
    let directory = path.parent().ok_or("generation directory")?;
    let mut csv = csv::Reader::from_path(directory.join("best-results-co2027.csv"))?;
    check!(csv
        .headers()?
        .iter()
        .any(|name| name == "canonical_candidate_id"));
    check!(eq; csv.records().collect::<Result<Vec<_>, _>>()?.len(), 0);
    check!(std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?.is_empty());
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest["schema_revision"], 8);
    let mut exclusions = csv::Reader::from_path(directory.join("participation_exclusions.csv"))?;
    let headers = exclusions.headers()?.clone();
    let rows = exclusions.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("withheld mark")?;
    let reasons =
        serde_json::to_string(&["high_school_status_missing", "summer_boundary_unresolved"])?;
    for (name, value) in [
        ("performance_id", expected.performance.as_str()),
        ("retained_candidate_id", expected.athlete.as_str()),
        ("disposition", "unresolved"),
        ("reasons", reasons.as_str()),
    ] {
        let index = headers
            .iter()
            .position(|header| header == name)
            .ok_or("exclusion column")?;
        check!(eq; row.get(index), Some(value));
    }
    workbook(path, expected)
}

fn workbook(path: &Path, expected: &fixture::Fixture) -> TestResult {
    use calamine::Reader;
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    check!(eq; book.worksheet_range("PRs")?.height(), 1);
    let range = book.worksheet_range("Athletes")?;
    check!(eq; range.height(), 2);
    let mut rows = range.rows();
    let headers = rows.next().ok_or("athlete header")?;
    let row = rows.next().ok_or("athlete row")?;
    for (name, value) in [
        ("Athlete ID", expected.public_id.as_str()),
        ("School", "Synthetic selected school"),
        ("State", "AL"),
    ] {
        let index = headers
            .iter()
            .position(|header| header.to_string() == name)
            .ok_or("athlete column")?;
        check!(eq; row.get(index).ok_or("athlete cell")?.to_string(), value);
    }
    Ok(())
}
