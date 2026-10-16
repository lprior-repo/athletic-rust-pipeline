use super::{CanonicalPerformance, TestResult};
use std::collections::BTreeMap;
use std::path::Path;

mod provenance;
mod selected;
mod workbook;

pub(super) struct Expected {
    pub public_id: String,
    pub school_id: String,
    pub eligible: CanonicalPerformance,
    pub excluded: CanonicalPerformance,
    pub evidence_refs: Vec<String>,
    pub eligible_evidence_refs: Vec<String>,
    pub packets: BTreeMap<String, serde_json::Value>,
    pub input_generation: String,
}

pub(super) fn artifacts(directory: &Path, workbook: &Path, expected: &Expected) -> TestResult {
    super::captures::high_school_results(directory, expected)?;
    selected::artifacts(directory, expected)?;
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    check!(eq; athletes.len(), 1);
    fields(
        athletes.first().ok_or("accepted public person")?,
        &[
            ("athlete_id", &expected.public_id),
            ("performance_availability", "eligible_selected_mark"),
        ],
    )?;
    provenance::retained(directory, expected)?;
    exclusions(directory, expected)?;
    workbook::pr(workbook, expected)
}

fn exclusions(directory: &Path, expected: &Expected) -> TestResult {
    let rows = csv_rows(&directory.join("participation_exclusions.csv"))?;
    check!(eq; rows.len(), 1);
    fields(
        rows.first()
            .ok_or("stronger result retained as exact exclusion")?,
        &[
            ("retained_candidate_id", expected.excluded.athlete.as_str()),
            ("performance_id", expected.excluded.id.as_str()),
            ("event_id", expected.excluded.event.as_str()),
            ("disposition", "excluded"),
            ("reasons", "[\"after_performance_window\"]"),
            ("diagnostics", "[]"),
            (
                "evidence_refs",
                &serde_json::to_string(&expected.evidence_refs)?,
            ),
            ("input_generation", &expected.input_generation),
        ],
    )?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    for (name, count) in [
        ("best_marks.csv", 1),
        ("participation_exclusions.csv", 1),
        ("comparison_classes.csv", 1),
    ] {
        check!(eq; manifest.pointer(&format!("/normalized_csv_rows/{name}")).and_then(serde_json::Value::as_u64), Some(count));
    }
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

pub(super) fn proofs(dataset: &super::ExportDataset, expected: &Expected) -> TestResult {
    for (performance, refs) in [
        (&expected.eligible, &expected.eligible_evidence_refs),
        (&expected.excluded, &expected.evidence_refs),
    ] {
        check!(eq; dataset.verified_participation()?.assessment(&performance.id)
            .ok_or("replayed exact independent proof set")?.evidence_refs(), refs.as_slice());
    }
    Ok(())
}
