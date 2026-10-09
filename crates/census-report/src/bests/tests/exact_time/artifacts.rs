use super::*;
use calamine::{open_workbook, Data, Reader, Xlsx};
use std::path::Path;

pub(super) fn assert_artifacts(
    store: &Store,
    out: &Path,
    expected: (&str, f64),
    nanos: i64,
    precision: u8,
) -> TestResult {
    let (expected, normalized) = expected;
    let dataset = ExportDataset::load(store)?;
    let rows = selected(&dataset);
    let selected = rows.first().ok_or("missing selected PR")?;
    check!(eq; rows.len(), 1);
    check!(eq; selected.result.value, nanos);
    check!(eq; selected.mark_text(), expected);
    let Mark::TimeSeconds(time) = &selected.result.mark else {
        return Err("best is not a time".into());
    };
    check!(eq; time.precision(), precision);
    let (jsonl, csv_path) = crate::bests::write(out, &rows, "co2027")?;
    let value: serde_json::Value = serde_json::from_str(std::fs::read_to_string(jsonl)?.trim())?;
    check!(eq; value.pointer("/mark/TimeSeconds/nanoseconds"), Some(&serde_json::json!(nanos)));
    check!(eq; value.pointer("/mark/TimeSeconds/precision"), Some(&serde_json::json!(precision)));
    check!(eq; value.get("value"), Some(&serde_json::json!(nanos)));
    check!(eq; value.get("normalized"), Some(&serde_json::json!(normalized)));
    assert_csv(&csv_path, expected, nanos)?;
    assert_xlsx(store, out, expected, normalized)
}

fn assert_csv(path: &Path, expected: &str, nanos: i64) -> TestResult {
    let mut csv = csv::Reader::from_path(path)?;
    let headers = csv.headers()?.clone();
    let mut records = csv.records();
    let record = records.next().ok_or("missing CSV best")??;
    let column = |name: &str| {
        headers
            .iter()
            .position(|header| header == name)
            .ok_or_else(|| format!("missing {name}"))
    };
    check!(eq; record.get(column("best_mark")?), Some(expected));
    check!(eq; record.get(column("best_value")?), Some(nanos.to_string().as_str()));
    check!(eq; record.get(column("best_value_unit")?), Some("ns"));
    match records.next() {
        Some(result) => Err(format!("unexpected extra best-results record: {:?}", result?).into()),
        None => Ok(()),
    }
}

fn assert_xlsx(store: &Store, out: &Path, expected: &str, normalized: f64) -> TestResult {
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: Some(out.join("publication")),
        limit: None,
        scope: Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?,
    };
    let workbook_path = crate::workbook::build(store, &options)?;
    crate::workbook::verify::verify_frozen(&workbook_path, &ExportDataset::load(store)?, &options)?;
    let mut workbook: Xlsx<_> = open_workbook(workbook_path)?;
    let range = workbook.worksheet_range("PRs")?;
    let mut rows = range.rows();
    let header = rows.next().ok_or("missing XLSX header")?;
    let column = |name: &str| {
        header
            .iter()
            .position(|value| value == name)
            .ok_or_else(|| format!("missing XLSX {name}"))
    };
    let row = rows.next().ok_or("missing XLSX best")?;
    check!(eq; row.get(column("Calculated PR")?).map(ToString::to_string), Some(expected.to_string()));
    check!(eq; row.get(column("Mark Value")?), Some(&Data::Float(normalized)));
    check!(eq; row.get(column("Unit")?).map(ToString::to_string), Some("s".to_string()));
    check!(eq; row.get(column("Result URL")?).map(ToString::to_string), Some("https://example.test/exact-time".to_string()));
    check!(eq; rows.next(), None);
    Ok(())
}
