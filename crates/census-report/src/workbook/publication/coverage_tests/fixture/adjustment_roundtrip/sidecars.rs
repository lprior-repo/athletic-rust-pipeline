use super::*;
use serde_json::Value;
use std::path::Path;

const CSV: &str = "best-results-co2027.csv";
const JSONL: &str = "best-results-co2027.jsonl";

fn verify_semantics(directory: &Path, captured: &Captured) -> TestResult {
    let revision = crate::workbook::publication::selected::revision(&captured.dataset)?;
    verify_sidecars::verify(directory, &captured.dataset, &captured.options, revision)?;
    Ok(())
}

fn metadata_refused(directory: &Path, captured: &Captured, name: &str) -> TestResult {
    let revision = crate::workbook::publication::selected::revision(&captured.dataset)?;
    check!(
        matches!(
            verify_sidecars::verify(directory, &captured.dataset, &captured.options, revision),
            Err(ReportError::Invariant { .. })
        ),
        "mutation {name}"
    );
    Ok(())
}

fn rewrite_csv(path: &Path, original: &[u8], column: &str, forged: &str) -> TestResult {
    let mut reader = csv::Reader::from_reader(original);
    let headers = reader.headers()?.clone();
    let index = headers
        .iter()
        .position(|header| header == column)
        .ok_or("CSV column missing")?;
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    let mut row = records
        .first()
        .ok_or("CSV best missing")?
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    *row.get_mut(index).ok_or("CSV cell missing")? = forged.into();
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer.write_record(&headers)?;
    writer.write_record(&row)?;
    std::fs::write(path, writer.into_inner()?)?;
    Ok(())
}

fn check_sidecar_metadata(directory: &Path, captured: &Captured) -> TestResult {
    let mut reader = csv::Reader::from_path(directory.join(CSV))?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    let record = records.first().ok_or("CSV adjusted best missing")?;
    for (name, expected) in [
        ("best_mark", "52.00"),
        ("original_mark", ""),
        ("best_value", "52000000000"),
        ("adjustment_kind", ""),
        ("adjustment_basis", ""),
        ("adjustment_note", ""),
        ("adjustment_source", ""),
    ] {
        let index = headers
            .iter()
            .position(|header| header == name)
            .ok_or("metadata header missing")?;
        check!(eq; record.get(index), Some(expected));
    }
    let text = std::fs::read_to_string(directory.join(JSONL))?;
    check!(eq; text.lines().count(), 1);
    let found: Value = serde_json::from_str(text.trim())?;
    let rows = selected_bests(&captured.dataset)?;
    check!(eq; found.get("schema").and_then(Value::as_str), Some("selected_identity_v2"));
    let selection = serde_json::to_value(rows.first().ok_or("physical selection missing")?)?;
    check!(eq; found.get("selection"), Some(&selection));
    verify_semantics(directory, captured)
}

pub(super) fn sidecar_forgeries_are_refused(captured: &Captured) -> TestResult {
    let scratch = tempfile::tempdir()?;
    let generation = captured
        .workbook
        .parent()
        .ok_or("publication directory missing")?;
    for entry in std::fs::read_dir(generation)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == std::ffi::OsStr::new("workbook.xlsx") || !entry.file_type()?.is_file() {
            continue;
        }
        std::fs::copy(entry.path(), scratch.path().join(&name))?;
    }
    check_sidecar_metadata(scratch.path(), captured)?;
    let csv_path = scratch.path().join(CSV);
    let original_csv = std::fs::read(&csv_path)?;
    for (column, forged) in [
        ("original_mark", "53.00"),
        ("adjustment_kind", "track_size_conversion"),
        ("adjustment_basis", "unknown"),
        ("adjustment_note", "forged conversion note"),
        (
            "adjustment_source",
            "{\"id\":\"foreign\",\"url\":\"https://example.test/foreign\"}",
        ),
    ] {
        rewrite_csv(&csv_path, &original_csv, column, forged)?;
        metadata_refused(scratch.path(), captured, CSV)?;
    }
    std::fs::write(&csv_path, original_csv)?;
    let jsonl_path = scratch.path().join(JSONL);
    let original_jsonl = std::fs::read_to_string(&jsonl_path)?;
    for (field, forged) in [
        ("original_mark", serde_json::to_value(seconds("53.00")?)?),
        (
            "adjustment_kind",
            serde_json::json!("track_size_conversion"),
        ),
        ("adjustment_basis", serde_json::json!("unknown")),
        (
            "adjustment_note",
            serde_json::json!("forged conversion note"),
        ),
        (
            "adjustment_source",
            serde_json::to_value(SourceRef::new(
                "foreign",
                Some("https://example.test/foreign".into()),
            ))?,
        ),
    ] {
        let mut found: Value = serde_json::from_str(original_jsonl.trim())?;
        found
            .get_mut("selection")
            .and_then(Value::as_object_mut)
            .ok_or("JSONL best object missing")?
            .insert(field.into(), forged);
        std::fs::write(&jsonl_path, format!("{}\n", serde_json::to_string(&found)?))?;
        metadata_refused(scratch.path(), captured, JSONL)?;
    }
    std::fs::write(jsonl_path, original_jsonl)?;
    check_sidecar_metadata(scratch.path(), captured)
}
