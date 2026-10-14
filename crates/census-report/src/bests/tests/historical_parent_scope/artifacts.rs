use super::*;
use calamine::{Data, Range};
use serde_json::Value;
use std::path::Path;

pub(super) fn golden(section: &str) -> TestResult<serde_json::Map<String, Value>> {
    let input = include_str!("expected-publication.json");
    check!(input.len() <= 8192);
    let root: Value = serde_json::from_str(input)?;
    root.get(section)
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| format!("missing frozen expected section {section}").into())
}

pub(super) fn json_text(row: &Value, fields: &[(&str, &str)]) -> TestResult {
    fields
        .iter()
        .try_for_each(|(field, expected)| -> TestResult {
            check!(eq; row.get(field).and_then(Value::as_str), Some(*expected), "{field}");
            Ok(())
        })
}

pub(super) fn csv_rows(path: &Path) -> TestResult<(csv::StringRecord, Vec<csv::StringRecord>)> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let rows = reader.records().take(9).collect::<Result<Vec<_>, _>>()?;
    check!(rows.len() <= 8, "unbounded synthetic CSV fixture");
    Ok((headers, rows))
}

fn csv_cells(
    headers: &csv::StringRecord,
    row: &csv::StringRecord,
    fields: &[(&str, &str)],
) -> TestResult {
    fields
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            let column = headers
                .iter()
                .position(|name| name == *header)
                .ok_or_else(|| format!("missing CSV column {header}"))?;
            check!(eq; row.get(column), Some(*expected), "{header}");
            Ok(())
        })
}

pub(super) fn csv_single(path: &Path, fields: &[(&str, &str)]) -> TestResult {
    let (headers, rows) = csv_rows(path)?;
    check!(eq; rows.len(), 1);
    csv_cells(
        &headers,
        rows.first().ok_or("missing single synthetic CSV row")?,
        fields,
    )
}

pub(super) fn assert_bests(directory: &Path, fixture: &Fixture, has_pr: bool) -> TestResult {
    let (headers, rows) = csv_rows(&directory.join("best-results-co2027.csv"))?;
    check!(eq; rows.len(), usize::from(has_pr));
    let lines = std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?;
    let lines = lines.lines().take(9).collect::<Vec<_>>();
    check!(eq; lines.len(), usize::from(has_pr));
    if !has_pr {
        return Ok(());
    }
    let csv = rows.first().ok_or("missing retained-parent CSV PR")?;
    csv_cells(
        &headers,
        csv,
        &[
            ("athlete_id", fixture.athlete.id.as_str()),
            ("name", fixture.case.athlete_name()),
            ("school", fixture.case.school_text()),
            ("state", fixture.case.school_bucket().code()),
            ("meet", fixture.case.meet_name()),
            ("performance_id", fixture.performance.id.as_str()),
            ("source_athlete", fixture.case.owner()),
            ("source_key", fixture.performance.source_key.as_str()),
            ("profile_url", PROFILE_URL),
        ],
    )?;
    golden("csv")?
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            csv_cells(
                &headers,
                csv,
                &[(
                    header.as_str(),
                    expected.as_str().ok_or("nontext CSV oracle")?,
                )],
            )
        })?;
    let row: Value =
        serde_json::from_str(lines.first().ok_or("missing retained-parent JSONL PR")?)?;
    assert_jsonl(&row, fixture)
}

fn assert_jsonl(row: &Value, fixture: &Fixture) -> TestResult {
    json_text(
        row,
        &[
            ("athlete", fixture.case.athlete_name()),
            ("athlete_school", fixture.case.school_text()),
            ("athlete_state", fixture.case.school_bucket().code()),
            ("meet_id", fixture.meet.id.as_str()),
            ("meet_state", fixture.case.venue_state().code()),
            ("meet", fixture.case.meet_name()),
            ("performance_id", fixture.performance.id.as_str()),
            ("source_athlete", fixture.case.owner()),
            ("source_key", fixture.performance.source_key.as_str()),
            ("result_url", RESULT_URL),
            ("profile_url", PROFILE_URL),
        ],
    )?;
    check!(eq; row.get("school"), Some(&serde_json::to_value(fixture.case.school())?));
    check!(eq; row.pointer("/key/athlete_id").and_then(Value::as_str), Some(fixture.athlete.id.as_str()));
    check!(eq; row.get("mark"), Some(&serde_json::to_value(Mark::TimeSeconds(ExactSeconds::parse("48.55")?))?));
    golden("jsonl")?
        .iter()
        .try_for_each(|(pointer, expected)| -> TestResult {
            check!(eq; row.pointer(pointer), Some(expected), "{pointer}");
            Ok(())
        })
}

pub(super) fn cell<'a>(range: &'a Range<Data>, name: &str) -> TestResult<&'a Data> {
    let headers = range.rows().next().ok_or("missing synthetic XLSX header")?;
    let column = headers
        .iter()
        .position(|cell| cell == name)
        .ok_or_else(|| format!("missing XLSX column {name}"))?;
    range
        .get((1, column))
        .ok_or_else(|| format!("missing XLSX data cell {name}").into())
}

pub(super) fn xlsx_single(range: &Range<Data>, fields: &[(&str, &str)]) -> TestResult {
    check!(eq; range.height(), 2);
    fields
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            check!(eq; cell(range, header)?.to_string(), *expected, "{header}");
            Ok(())
        })
}

pub(super) fn xlsx_golden(range: &Range<Data>, section: &str) -> TestResult {
    golden(section)?
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            xlsx_single(
                range,
                &[(
                    header.as_str(),
                    expected.as_str().ok_or("nontext XLSX oracle")?,
                )],
            )
        })
}

pub(super) fn bound_generation(directory: &Path) -> TestResult {
    let mut bytes = 0_u64;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let size = entry.metadata()?.len();
        check!(size <= 1_048_576);
        bytes = bytes
            .checked_add(size)
            .ok_or("synthetic artifact byte counter exhausted")?;
    }
    check!(bytes <= 2_097_152);
    Ok(())
}
