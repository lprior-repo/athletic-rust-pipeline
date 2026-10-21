use super::*;
use crate::milesplit::parse_raw_located_bound;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use census_domain::model::{EventKind, GradYear, SchoolYear};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"
));
const HEADERS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.headers"
));

fn capture() -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.with_timezone(&Utc),
        BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: HEADERS.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-10-16T00:06:38Z")?.with_timezone(&Utc),
        },
    )?)
}

fn locate(
    matches: impl Fn(&crate::result_file::ParsedEvent, &crate::result_file::ParsedRow) -> bool,
) -> TestResult<SourceRowLocator> {
    let mut locations = Vec::new();
    let parsed = parse_raw_located_bound(
        std::str::from_utf8(BODY)?,
        URL,
        &|_, _| Ok(()),
        &mut |locator, _| {
            locations.push(locator);
            Ok(())
        },
    )?;
    for locator in locations {
        let event = parsed
            .page
            .meet
            .events
            .get(locator.event_index)
            .ok_or("event")?;
        let row = event.rows.get(locator.row_index).ok_or("row")?;
        if matches(event, row) {
            return Ok(locator.row);
        }
    }
    Err("original source row not found".into())
}

#[test]
fn genuine_juniors_prove_only_the_original_result_day_and_cohort() -> TestResult {
    let capture = capture()?;
    for name in ["Owen Courser", "Brennan Smith"] {
        let locator = locate(|event, row| event.kind == EventKind::Track400m && row.name == name)?;
        let status = read(&capture, locator)?;
        check!(eq; status.subject.name(), name);
        check!(eq; status.subject.school(), "Detroit Catholic Central");
        check!(eq; status.day.as_str(), "2026-05-15");
        check!(eq; status.grade.get(), 11);
        check!(eq; GradYear::of(status.grade, SchoolYear::from_date(status.day.as_str()).ok_or("school year")?), Some(GradYear::CO2027));
        check!(eq; status.ownership, SubjectOwnershipV1::DocumentOnly(status.subject));
    }
    Ok(())
}
#[test]
fn status_subject_locator_bounds_the_literal_statement_not_the_entire_row() -> TestResult {
    let capture = capture()?;
    let locator =
        locate(|event, row| event.kind == EventKind::Track400m && row.name == "Owen Courser")?;
    let status = read(&capture, locator)?;
    let original = std::str::from_utf8(BODY)?;
    let row_start = locator.byte_offset;
    let row_end = row_start
        .checked_add(locator.byte_length)
        .ok_or("original row span overflow")?;
    let full_row = original
        .get(row_start..row_end)
        .ok_or("original row span")?;
    let bytes = status
        .subject
        .locator()
        .strip_prefix("html_bytes:")
        .ok_or("status locator prefix")?;
    let (start_text, end_text) = bytes.split_once(':').ok_or("status locator range")?;
    let start: usize = start_text
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let end: usize = end_text
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let statement = original.get(start..end).ok_or("student statement span")?;
    let literal_grade = statement
        .strip_prefix("Owen Courser")
        .and_then(|text| text.strip_suffix("Detroit Catholic Central"))
        .map(str::trim);
    check!(eq; literal_grade, Some("11"));
    check!(!statement.contains("51.27a"));
    check!(eq; full_row.contains("Owen Courser"), true);
    check!(eq; full_row.contains("51.27a"), true);
    check!(eq; full_row.len() > statement.len(), true);
    check!(eq; status.subject.document_namespace(), "milesplit_result_document_v1");
    check!(eq; status.subject.body_sha256(), hash(BODY));
    Ok(())
}

#[test]
fn distinct_athletes_from_same_school_produce_distinct_status_subjects() -> TestResult {
    let capture = capture()?;
    let owen =
        locate(|event, row| event.kind == EventKind::Track400m && row.name == "Owen Courser")?;
    let brennan =
        locate(|event, row| event.kind == EventKind::Track400m && row.name == "Brennan Smith")?;
    let owen_status = read(&capture, owen)?;
    let brennan_status = read(&capture, brennan)?;
    check!(eq; owen_status.subject.locator() != brennan_status.subject.locator(), true);
    check!(eq; owen_status.subject.reference() != brennan_status.subject.reference(), true);
    check!(eq; owen_status.subject.name(), "Owen Courser");
    check!(eq; brennan_status.subject.name(), "Brennan Smith");
    check!(eq; owen_status.subject.school(), brennan_status.subject.school());
    check!(eq; owen_status.grade, brennan_status.grade);
    check!(eq; owen_status.day, brennan_status.day);
    Ok(())
}

#[test]
fn foreign_locator_and_altered_original_capture_are_refused() -> TestResult {
    let capture = capture()?;
    let locator =
        locate(|event, row| event.kind == EventKind::Track400m && row.name == "Owen Courser")?;
    let original = read(&capture, locator)?;
    check!(eq; original.grade.get(), 11);
    let foreign = SourceRowLocator {
        ordinal: u32::MAX,
        byte_offset: 0,
        byte_length: 1,
    };
    check!(matches!(
        read(&capture, foreign),
        Err(census_store::StoreError::Refused { .. })
    ));
    let mut value = serde_json::to_value(&capture)?;
    *value
        .get_mut("body_sha256")
        .ok_or("capture body commitment")? = serde_json::json!("0".repeat(64));
    let altered: RetainedHtmlCaptureV1 = serde_json::from_value(value)?;
    check!(matches!(
        read(&altered, locator),
        Err(census_store::StoreError::Refused { .. })
    ));
    Ok(())
}

#[test]
fn another_schools_explicit_grade_cannot_claim_dcc_student_status() -> TestResult {
    let capture = capture()?;
    let locator = locate(|event, row| {
        event.kind == EventKind::Track400m
            && row.school != "Detroit Catholic Central"
            && row.grade.is_some()
    })?;
    check!(matches!(
        read(&capture, locator),
        Err(census_store::StoreError::Refused { detail })
            if detail == "original result school is outside the supported DCC status scope"
    ));
    Ok(())
}
