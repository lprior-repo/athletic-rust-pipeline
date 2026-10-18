use super::super::source::TestResult;
use crate::export::ExportDataset;
use calamine::{Reader, Xlsx};
use census_domain::model::CanonicalPerformance;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
pub(super) type Row = BTreeMap<String, String>;
pub(super) fn csv_rows(path: &Path) -> TestResult<Vec<Row>> {
    let bytes = read_bytes(path)?;
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader.headers()?.clone();
    check!(!headers.is_empty());
    check!(eq; headers.iter().collect::<BTreeSet<_>>().len(), headers.len());
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        check!(eq; record.len(), headers.len());
        check!(rows.len() < 128);
        rows.push(
            headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        );
    }
    Ok(rows)
}
pub(super) fn headers_of(path: &Path) -> TestResult<Vec<String>> {
    let bytes = read_bytes(path)?;
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader.headers()?.clone();
    check!(!headers.is_empty());
    Ok(headers.iter().map(str::to_owned).collect())
}
pub(super) fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult<()> {
    for (name, value) in expected {
        check!(eq; row.get(*name).map(String::as_str), Some(*value), "field {name}");
    }
    Ok(())
}
pub(super) fn unique<'a>(rows: &'a [Row], key: &str, value: &str) -> TestResult<&'a Row> {
    let mut found = rows
        .iter()
        .filter(|row| row.get(key).map(String::as_str) == Some(value));
    let row = found
        .next()
        .ok_or_else(|| format!("missing {key}={value}"))?;
    check!(found.next().is_none(), "duplicate {key}={value}");
    Ok(row)
}
pub(super) fn directory_of(workbook: &Path) -> TestResult<PathBuf> {
    Ok(workbook
        .parent()
        .ok_or("published generation")?
        .to_path_buf())
}
pub(super) fn read_bytes(path: &Path) -> TestResult<Vec<u8>> {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    Ok(std::fs::read(path)?)
}
pub(super) fn manifest(directory: &Path) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&read_bytes(
        &directory.join("manifest.json"),
    )?)?)
}
pub(super) fn reopened(directory: &Path) -> TestResult<ExportDataset> {
    let manifest = manifest(directory)?;
    let digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("published frozen input commitment missing")?;
    Ok(ExportDataset::reopen_frozen_with_digest(
        &directory.join("frozen-input.json"),
        digest,
    )?)
}
pub(super) fn outcome_of(dataset: &ExportDataset) -> TestResult<crate::bests::SelectionOutcome> {
    let profile = dataset.selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("selected profile requires one cohort".into());
    };
    Ok(crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(cohort.get()),
            limit: None,
            jurisdictions: profile.input().jurisdictions.clone(),
        },
    )?)
}
pub(super) fn by_mark(
    dataset: &ExportDataset,
    seconds: &str,
    day: &str,
) -> TestResult<CanonicalPerformance> {
    let mut found = dataset
        .performances
        .iter()
        .filter(|row| crate::bests::mark_text(&row.mark) == seconds && row.date == day);
    let row = found
        .next()
        .ok_or_else(|| format!("missing controlled mark {seconds} on {day}"))?;
    check!(
        found.next().is_none(),
        "duplicate controlled mark {seconds}"
    );
    Ok(row.clone())
}
pub(super) fn sheet(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> TestResult<Vec<Row>> {
    let range = book.worksheet_range(name)?;
    check!(range.height() <= 128);
    check!(range.width() <= 128);
    let mut rows = range.rows();
    let headers: Vec<String> = rows
        .next()
        .ok_or("workbook headers")?
        .iter()
        .map(ToString::to_string)
        .collect();
    check!(!headers.is_empty());
    check!(eq; headers.iter().collect::<BTreeSet<_>>().len(), headers.len());
    rows.map(|row| {
        check!(eq; row.len(), headers.len());
        Ok(headers
            .iter()
            .cloned()
            .zip(row.iter().map(ToString::to_string))
            .collect())
    })
    .collect()
}
