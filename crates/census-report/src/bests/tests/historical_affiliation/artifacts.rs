use super::*;
use serde_json::Value;
use std::path::Path;

pub(super) fn csv_row(path: &Path) -> TestResult<(csv::StringRecord, csv::StringRecord)> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let mut rows = reader.records();
    let row = rows
        .next()
        .ok_or("missing historical-affiliation CSV row")??;
    match rows.next() {
        Some(extra) => {
            Err(format!("unexpected historical-affiliation CSV row: {:?}", extra?).into())
        }
        None => Ok((headers, row)),
    }
}

pub(super) fn csv_cells(path: &Path, fields: &[(&str, &str)]) -> TestResult {
    let (headers, row) = csv_row(path)?;
    fields
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            let column = headers
                .iter()
                .position(|name| name == *header)
                .ok_or_else(|| format!("missing historical-affiliation CSV column {header}"))?;
            check!(eq; row.get(column), Some(*expected), "{header}");
            Ok(())
        })
}

pub(super) fn assert_bests(directory: &Path, fixture: &Fixture) -> TestResult {
    csv_cells(
        &directory.join("best-results-co2027.csv"),
        &[
            ("athlete_id", fixture.athlete.id.as_str()),
            ("name", ATHLETE_NAME),
            ("school", fixture.case.school_text()),
            ("state", fixture.case.state().code()),
            ("grad_year", "2027"),
            ("best_mark", "48.55"),
            ("best_value", "48550000000"),
            ("best_value_unit", "ns"),
            ("date", RESULT_DATE),
            ("meet", fixture.meet.name.as_str()),
            ("performance_id", fixture.performance.id.as_str()),
            ("source_athlete", SOURCE_OWNER),
            ("source_key", SOURCE_KEY),
            ("profile_url", PROFILE_URL),
            ("marks_in_event", "1"),
        ],
    )?;
    assert_jsonl(&directory.join("best-results-co2027.jsonl"), fixture)
}

fn assert_jsonl(path: &Path, fixture: &Fixture) -> TestResult {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let line = lines
        .next()
        .ok_or("missing historical-affiliation JSONL row")?;
    check!(eq; lines.next(), None);
    let found: Value = serde_json::from_str(line)?;
    check!(eq; found.get("school"), Some(&serde_json::to_value(fixture.case.school())?));
    [
        ("athlete_school", fixture.case.school_text()),
        ("athlete_state", fixture.case.state().code()),
        ("athlete", ATHLETE_NAME),
        ("profile_url", PROFILE_URL),
        ("performance_id", fixture.performance.id.as_str()),
        ("meet_id", fixture.meet.id.as_str()),
        ("meet_state", "MI"),
        ("date", RESULT_DATE),
        ("meet", fixture.meet.name.as_str()),
        ("source_athlete", SOURCE_OWNER),
        ("source_key", SOURCE_KEY),
        ("result_url", RESULT_URL),
        ("heat", "2"),
        ("round", "Final"),
    ]
    .into_iter()
    .try_for_each(|(field, expected)| -> TestResult {
        check!(eq; found.get(field).and_then(Value::as_str), Some(expected), "{field}");
        Ok(())
    })?;
    check!(eq; found.pointer("/key/athlete_id").and_then(Value::as_str), Some(fixture.athlete.id.as_str()));
    check!(eq; found.get("mark"), Some(&serde_json::to_value(&fixture.performance.mark)?));
    check!(eq; found.get("value").and_then(Value::as_i64), Some(MARK_VALUE));
    check!(eq; found.get("normalized").and_then(Value::as_f64), Some(48.55));
    check!(eq; found.pointer("/population/marks").and_then(Value::as_u64), Some(1));
    check!(eq; found.pointer("/population/sources").and_then(Value::as_u64), Some(1));
    check!(eq; found.get("conflicts"), Some(&serde_json::json!([])));
    Ok(())
}
