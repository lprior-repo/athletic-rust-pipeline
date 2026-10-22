use super::*;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};

type Row = BTreeMap<String, String>;

pub(super) fn independent_positive(
    workbook: &Path,
    dataset: &ExportDataset,
    source: &fixture::Capture,
    public: &str,
) -> TestResult<Value> {
    let directory = workbook.parent().ok_or("controlled generation")?;
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    let recruiting = csv_rows(&directory.join("recruiting.csv"))?;
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    let worksheet = sheet_rows(workbook)?;
    check!(eq; athletes.len(), 1);
    check!(eq; recruiting.len(), 1);
    check!(eq; worksheet.len(), 1);
    check!(eq; affiliations.len(), dataset.athletes.len());
    check!(eq; dataset.athletes.len(), 1);
    check!(dataset.performances.is_empty());
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_id(source.athlete.id.as_str()).map(|id| id.as_str()), Some(public));
    let audit = dataset
        .verified_current_affiliations()?
        .audit(&source.athlete.id)
        .ok_or("controlled affiliation audit")?;
    let affiliation = unique(
        &affiliations,
        "retained_candidate_id",
        source.athlete.id.as_str(),
    )?;
    fields(
        affiliation,
        &[
            ("resolution_status", "resolved_at_selected_cutoff"),
            ("current_school_id", source.school.id.as_str()),
            ("as_of", "2026-10-16"),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
        ],
    )?;
    check!(eq; affiliation.get("profile_id").map(String::as_str), Some(dataset.selected_profile_id().ok_or("selected profile")?.as_str()));
    check!(eq; affiliation.get("evidence_refs"), Some(&serde_json::to_string(audit.references())?));
    check!(eq; affiliation.get("unavailable_reasons"), Some(&serde_json::to_string(audit.unavailable())?));
    let row = athletes.first().ok_or("nonempty athlete")?;
    fields(
        row,
        &[
            ("athlete_id", public),
            ("canonical_candidate_id", source.athlete.id.as_str()),
            ("current_school_id", source.school.id.as_str()),
            ("current_school_status", "resolved_at_selected_cutoff"),
            ("identity_status", "accepted_public_identity"),
            (
                "affiliation_evidence_status",
                "retained_selected_affiliation_evidence",
            ),
        ],
    )?;
    let recruiting_row = recruiting.first().ok_or("nonempty recruiting")?;
    fields(
        recruiting_row,
        &[
            ("athlete_id", public),
            ("canonical_candidate_id", source.athlete.id.as_str()),
            ("school", source.school.name.as_str()),
            ("state", "MI"),
        ],
    )?;
    let sheet_row = worksheet.first().ok_or("nonempty sheet")?;
    fields(
        sheet_row,
        &[
            ("Athlete ID", public),
            ("School ID", source.school.id.as_str()),
            ("School", source.school.name.as_str()),
            ("School State", "MI"),
        ],
    )?;
    let manifest: Value = serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/athletes.csv").and_then(Value::as_u64), Some(1));
    check!(eq; manifest.pointer("/normalized_csv_rows/current_affiliations.csv").and_then(Value::as_u64), Some(u64::try_from(dataset.athletes.len())?));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(Value::as_u64), Some(0));
    Ok(
        json!({"athletes": athletes, "recruiting": recruiting, "affiliations": affiliations, "worksheet": worksheet}),
    )
}

pub(super) fn independent_withdrawn(
    workbook: &Path,
    dataset: &ExportDataset,
    source: &fixture::Capture,
) -> TestResult<Value> {
    let directory = workbook.parent().ok_or("withdrawn generation")?;
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    let recruiting = csv_rows(&directory.join("recruiting.csv"))?;
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    let worksheet = sheet_rows(workbook)?;
    check!(athletes.is_empty());
    check!(recruiting.is_empty());
    check!(worksheet.is_empty());
    check!(eq; affiliations.len(), dataset.athletes.len());
    check!(eq; affiliations.len(), 1);
    let audit = dataset
        .verified_current_affiliations()?
        .audit(&source.athlete.id)
        .ok_or("withdrawn affiliation audit")?;
    let affiliation = unique(
        &affiliations,
        "retained_candidate_id",
        source.athlete.id.as_str(),
    )?;
    fields(
        affiliation,
        &[
            ("resolution_status", "current_affiliation_unknown"),
            ("current_school_id", ""),
            ("as_of", "2026-10-16"),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
        ],
    )?;
    check!(eq; affiliation.get("profile_id").map(String::as_str), Some(dataset.selected_profile_id().ok_or("selected profile")?.as_str()));
    check!(eq; affiliation.get("evidence_refs"), Some(&serde_json::to_string(audit.references())?));
    check!(eq; affiliation.get("unavailable_reasons"), Some(&serde_json::to_string(audit.unavailable())?));
    check!(eq; audit.unavailable(), &["reviewed_roster_member_withdrawn".to_string()]);
    let manifest: Value = serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/athletes.csv").and_then(Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/current_affiliations.csv").and_then(Value::as_u64), Some(1));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(Value::as_u64), Some(0));
    Ok(
        json!({"athletes": athletes, "recruiting": recruiting, "affiliations": affiliations, "worksheet": worksheet}),
    )
}

fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (key, value) in expected {
        check!(eq; row.get(*key).map(String::as_str), Some(*value), "{key}");
    }
    Ok(())
}

fn unique<'a>(rows: &'a [Row], key: &str, value: &str) -> TestResult<&'a Row> {
    let mut selected = rows
        .iter()
        .filter(|row| row.get(key).map(String::as_str) == Some(value));
    let row = selected.next().ok_or("controlled output row")?;
    check!(selected.next().is_none());
    Ok(row)
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
    check!(eq; workbook.sheet_names(), vec!["Athletes".to_owned()]);
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
