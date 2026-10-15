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
    check!(eq; reply["qualification_lifecycle"], 6);
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
