use super::{CanonicalPerformance, TestResult};
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::BTreeMap;
use std::path::Path;

mod provenance;

pub(super) struct Expected {
    pub public_id: String,
    pub unresolved: CanonicalPerformance,
    pub excluded: CanonicalPerformance,
    pub evidence_refs: Vec<String>,
    pub unresolved_evidence_refs: Vec<String>,
    pub input_generation: String,
}

pub(super) fn artifacts(directory: &Path, workbook: &Path, expected: &Expected) -> TestResult {
    super::captures::high_school_results(directory)?;
    for name in ["best_marks.csv", "best-results-co2027.csv"] {
        check!(csv_rows(&directory.join(name))?.is_empty());
    }
    check!(std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?.is_empty());
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    check!(eq; athletes.len(), 1);
    fields(
        athletes.first().ok_or("accepted public person")?,
        &[
            ("athlete_id", &expected.public_id),
            (
                "performance_availability",
                "no_eligible_selected_mark_in_snapshot",
            ),
        ],
    )?;
    provenance::retained(directory, expected)?;
    exclusions(directory, expected)?;
    workbook_pr(workbook, expected)
}

fn exclusions(directory: &Path, expected: &Expected) -> TestResult {
    let rows = csv_rows(&directory.join("participation_exclusions.csv"))?;
    check!(eq; rows.len(), 2);
    let reasons = serde_json::to_string(&[
        "after_performance_window",
        "high_school_status_missing",
        "summer_boundary_unresolved",
    ])?;
    let evidence_refs = serde_json::to_string(&expected.evidence_refs)?;
    fields(
        rows.iter()
            .find(|row| {
                row.get("performance_id").map(String::as_str) == Some(expected.excluded.id.as_str())
            })
            .ok_or("retained exact configured-window exclusion")?,
        &[
            ("retained_candidate_id", expected.excluded.athlete.as_str()),
            ("performance_id", expected.excluded.id.as_str()),
            ("event_id", expected.excluded.event.as_str()),
            ("disposition", "excluded"),
            ("reasons", &reasons),
            ("diagnostics", "[]"),
            ("evidence_refs", &evidence_refs),
            ("input_generation", &expected.input_generation),
        ],
    )?;
    let missing = rows
        .iter()
        .find(|row| {
            row.get("performance_id").map(String::as_str) == Some(expected.unresolved.id.as_str())
        })
        .ok_or("retained missing-status/window result")?;
    let unresolved_refs = serde_json::to_string(&expected.unresolved_evidence_refs)?;
    fields(
        missing,
        &[
            (
                "retained_candidate_id",
                expected.unresolved.athlete.as_str(),
            ),
            ("event_id", expected.unresolved.event.as_str()),
            ("evidence_refs", &unresolved_refs),
            ("disposition", "unresolved"),
            (
                "reasons",
                "[\"high_school_status_missing\",\"summer_boundary_unresolved\"]",
            ),
            ("diagnostics", "[]"),
            ("input_generation", &expected.input_generation),
        ],
    )?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(serde_json::Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/participation_exclusions.csv").and_then(serde_json::Value::as_u64), Some(2));
    Ok(())
}

fn workbook_pr(path: &Path, expected: &Expected) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let range = book.worksheet_range("PRs")?;
    check!(eq; range.height(), 1);
    let header = range.rows().next().ok_or("PR headers")?;
    for label in [
        "Athlete ID",
        "Canonical Candidate ID",
        "Calculated PR",
        "Performance ID",
    ] {
        check!(header.iter().any(|cell| cell.to_string() == label));
    }
    let athletes = book.worksheet_range("Athletes")?;
    check!(athletes.rows().any(|row| row
        .iter()
        .any(|cell| cell.to_string() == expected.public_id)));
    Ok(())
}

fn fields(row: &BTreeMap<String, String>, expected: &[(&str, &str)]) -> TestResult {
    for (name, value) in expected {
        check!(eq; row.get(*name).map(String::as_str), Some(*value));
    }
    Ok(())
}

fn csv_rows(path: &Path) -> TestResult<Vec<BTreeMap<String, String>>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}
