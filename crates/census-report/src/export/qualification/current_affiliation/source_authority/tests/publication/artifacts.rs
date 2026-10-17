use super::*;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};

type Row = BTreeMap<String, String>;

pub(super) fn readback(
    workbook: &Path,
    dataset: &ExportDataset,
    expected: &Expected,
) -> TestResult<Value> {
    let directory = workbook
        .parent()
        .ok_or("controlled publication generation")?;
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    let recruiting = csv_rows(&directory.join("recruiting.csv"))?;
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    let worksheet = sheet_rows(workbook)?;
    check!(eq; athletes.len(), expected.selected_count());
    check!(eq; recruiting.len(), expected.selected_count());
    check!(eq; worksheet.len(), expected.selected_count());
    check!(eq; affiliations.len(), dataset.athletes.len());
    let audit = dataset
        .verified_current_affiliations()?
        .audit(&expected.athlete)
        .ok_or("controlled source audit")?;
    let affiliation = affiliations
        .iter()
        .find(|row| {
            row.get("retained_candidate_id").map(String::as_str) == Some(expected.athlete.as_str())
        })
        .ok_or("controlled retained affiliation")?;
    fields(
        affiliation,
        &[
            ("resolution_status", expected.status()),
            (
                "profile_id",
                dataset
                    .selected_profile_id()
                    .ok_or("selected profile")?
                    .as_str(),
            ),
            ("as_of", "2026-10-16"),
            ("input_generation", &dataset.lineage.input_generation),
        ],
    )?;
    check!(eq; affiliation.get("evidence_refs"), Some(&serde_json::to_string(audit.references())?));
    check!(eq; affiliation.get("unavailable_reasons"), Some(&serde_json::to_string(audit.unavailable())?));
    if let Some(school) = expected.school() {
        fields(affiliation, &[("current_school_id", school.as_str())])?;
        positive_rows(dataset, expected, &athletes, &recruiting, &worksheet)?;
    } else {
        fields(affiliation, &[("current_school_id", "")])?;
    }
    let manifest: Value = serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/athletes.csv").and_then(Value::as_u64), Some(u64::try_from(expected.selected_count())?));
    check!(eq; manifest.pointer("/normalized_csv_rows/current_affiliations.csv").and_then(Value::as_u64), Some(u64::try_from(dataset.athletes.len())?));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(Value::as_u64), Some(0));
    Ok(
        json!({"athletes":athletes,"recruiting":recruiting,"affiliations":affiliations,"worksheet":worksheet}),
    )
}

fn positive_rows(
    dataset: &ExportDataset,
    expected: &Expected,
    athletes: &[Row],
    recruiting: &[Row],
    worksheet: &[Row],
) -> TestResult {
    let public = dataset
        .verified_public_identities()?
        .public_id(expected.athlete.as_str())
        .ok_or("controlled public identifier")?;
    let row = athletes
        .first()
        .ok_or("required nonempty normalized athlete")?;
    fields(
        row,
        &[
            ("athlete_id", public.as_str()),
            ("canonical_candidate_id", expected.athlete.as_str()),
            ("current_school_id", expected.current_school.as_str()),
            ("current_school_status", expected.status()),
            ("identity_status", "accepted_public_identity"),
            (
                "affiliation_evidence_status",
                "retained_selected_affiliation_evidence",
            ),
        ],
    )?;
    let refs: Vec<String> = serde_json::from_str(
        row.get("affiliation_evidence_refs")
            .ok_or("normalized affiliation references")?,
    )?;
    check!(!refs.is_empty());
    fields(
        recruiting
            .first()
            .ok_or("required nonempty recruiting athlete")?,
        &[
            ("athlete_id", public.as_str()),
            ("canonical_candidate_id", expected.athlete.as_str()),
            ("school", &expected.school_name),
            ("state", "MI"),
        ],
    )?;
    fields(
        worksheet
            .first()
            .ok_or("required nonempty workbook athlete")?,
        &[
            ("Athlete ID", public.as_str()),
            ("Canonical Candidate ID", expected.athlete.as_str()),
            ("School ID", expected.current_school.as_str()),
            ("School", &expected.school_name),
            ("State", "MI"),
        ],
    )
}

fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (key, value) in expected {
        check!(eq; row.get(*key).map(String::as_str), Some(*value), "{key}");
    }
    Ok(())
}

fn csv_rows(path: &Path) -> TestResult<Vec<Row>> {
    check!(std::fs::metadata(path)?.len() <= 8 * 1024 * 1024);
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    check!(eq; header.iter().collect::<BTreeSet<_>>().len(), header.len());
    let mut rows = Vec::new();
    for record in reader.records() {
        check!(rows.len() < 128);
        let record = record?;
        check!(eq; record.len(), header.len());
        rows.push(
            header
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        );
    }
    Ok(rows)
}

fn sheet_rows(path: &Path) -> TestResult<Vec<Row>> {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let range = workbook.worksheet_range("Athletes")?;
    check!(range.height() <= 128 && range.width() <= 128);
    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or("controlled athlete worksheet header")?
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    check!(eq; header.iter().collect::<BTreeSet<_>>().len(), header.len());
    rows.map(|row| {
        check!(eq; row.len(), header.len());
        Ok(header
            .iter()
            .cloned()
            .zip(row.iter().map(ToString::to_string))
            .collect())
    })
    .collect()
}
