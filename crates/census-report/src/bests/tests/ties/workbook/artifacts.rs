use super::{Observation, SharedSelection, TestResult};
use serde_json::Value;
use std::path::Path;

pub(super) fn result_url(expected: &Observation) -> TestResult<&str> {
    expected
        .performance
        .evidence
        .first()
        .and_then(|evidence| evidence.source.url.as_deref())
        .ok_or_else(|| "best-results fixture has no result URL".into())
}

pub(super) fn csv_row(path: &Path) -> TestResult<(csv::StringRecord, csv::StringRecord)> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let mut rows = reader.records();
    let row = rows.next().ok_or("missing best-results CSV row")??;
    if let Some(extra) = rows.next() {
        return Err(format!("unexpected best-results CSV row: {:?}", extra?).into());
    }
    Ok((headers, row))
}

pub(super) fn jsonl_row(path: &Path) -> TestResult<Value> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let line = lines.next().ok_or("missing best-results JSONL row")?;
    check!(eq; lines.next(), None);
    Ok(serde_json::from_str(line)?)
}

fn assert_csv(path: &Path, expected: &Observation, row: &SharedSelection) -> TestResult {
    let (headers, found) = csv_row(path)?;
    let performance = &expected.performance;
    let mark = crate::bests::mark_text(&performance.mark);
    let value = row.result.value.to_string();
    let marks = row.population.marks.to_string();
    [
        ("athlete_id", performance.athlete.as_str()),
        ("performance_id", performance.id.as_str()),
        ("date", performance.date.as_str()),
        ("meet", expected.meet.name.as_str()),
        ("source_key", performance.source_key.as_str()),
        ("source_athlete", "ties-2027"),
        ("best_mark", mark.as_str()),
        ("best_value", value.as_str()),
        ("marks_in_event", marks.as_str()),
    ]
    .into_iter()
    .try_for_each(|(header, value)| -> TestResult {
        let column = headers
            .iter()
            .position(|name| name == header)
            .ok_or_else(|| format!("missing best-results CSV column {header}"))?;
        check!(eq; found.get(column), Some(value), "{header}");
        Ok(())
    })?;
    Ok(())
}

fn assert_jsonl(path: &Path, expected: &Observation, row: &SharedSelection) -> TestResult {
    let found = jsonl_row(path)?;
    let performance = &expected.performance;
    [
        ("performance_id", performance.id.as_str()),
        ("date", performance.date.as_str()),
        ("meet", expected.meet.name.as_str()),
        ("meet_id", performance.meet.as_str()),
        ("source_key", performance.source_key.as_str()),
        ("source_athlete", "ties-2027"),
        ("result_url", result_url(expected)?),
    ]
    .into_iter()
    .try_for_each(|(field, value)| -> TestResult {
        check!(eq; found.get(field).and_then(Value::as_str), Some(value), "{field}");
        Ok(())
    })?;
    check!(eq; found.pointer("/key/athlete_id").and_then(Value::as_str), Some(performance.athlete.as_str()));
    check!(eq; found.get("mark"), Some(&serde_json::to_value(&performance.mark)?));
    check!(eq; found.get("value").and_then(Value::as_i64), Some(row.result.value));
    check!(eq; found.pointer("/population/marks").and_then(Value::as_u64), Some(u64::try_from(row.population.marks)?));
    check!(eq; found, serde_json::to_value(row)?);
    Ok(())
}

pub(super) fn assert_bests(
    directory: &Path,
    expected: &Observation,
    row: &SharedSelection,
) -> TestResult {
    assert_csv(&directory.join("best-results-co2027.csv"), expected, row)?;
    assert_jsonl(&directory.join("best-results-co2027.jsonl"), expected, row)
}
