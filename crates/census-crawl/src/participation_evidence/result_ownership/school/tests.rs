use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::DateTime;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const BODY: &[u8] = include_bytes!("../tests/fixtures/milesplit-result.html");

fn capture(body: &[u8]) -> TestResult<RetainedHtmlCaptureV1> {
    let at = DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        at,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: Vec::new(),
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: at,
        },
    )?)
}

fn available<'a>(
    row: &CheckedRetainedResultRowV1<'a>,
) -> TestResult<CheckedResultSchoolStatementV1<'a>> {
    match row.school_statement()? {
        ResultSchoolStatementV1::Available(value) => Ok(value),
        ResultSchoolStatementV1::Unavailable => Err("expected original school statement".into()),
    }
}

fn assert_original(document: &CheckedRetainedResultDocumentV1<'_>, index: usize) -> TestResult {
    let row = document.row(index).ok_or("checked row")?;
    let school = available(&row)?;
    check!(eq; document.original_document().get(school.span()), Some(school.text()));
    check!(eq; school.text(), row.parsed_row().school);
    check!(eq; school.digest(), super::super::super::hash(school.text().as_bytes()));
    check!(eq; school.body_digest(), document.capture().body_sha256());
    check!(eq; school.row(), row.locator());
    check!(eq; school.row_digest(), row.original_row_digest());
    check!(eq; school.row_reference(), row.reference());
    check!(eq; school.context_reference(), document.context_reference());
    check!(school.span().start >= row.locator().byte_offset);
    check!(
        school.span().end
            <= row
                .locator()
                .byte_offset
                .checked_add(row.locator().byte_length)
                .ok_or("row end")?
    );
    check!(eq; school.interpretation(), INTERPRETATION);
    check!(ne; school.reference(), row.reference());
    check!(eq; row.reference(), serialized_digest(&(super::super::INTERPRETATION, URL, document.capture().body_sha256(), row.locator()))?);
    Ok(())
}

#[test]
fn genuine_school_statements_bind_three_original_rows_without_provider_school_identity(
) -> TestResult {
    let capture = capture(BODY)?;
    check!(eq; capture.body_sha256(), "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5");
    let rows = [(527, 80520, 100), (540, 81648, 92), (572, 84465, 92)].map(
        |(ordinal, byte_offset, byte_length)| SourceRowLocator {
            ordinal,
            byte_offset,
            byte_length,
        },
    );
    let document = read_retained_html_result_rows(&capture, &rows)?;
    let mut refs = std::collections::BTreeSet::new();
    for index in 0..3 {
        assert_original(&document, index)?;
        let row = document.row(index).ok_or("genuine row")?;
        let school = available(&row)?;
        check!(eq; school.text(), "Detroit Catholic Central");
        check!(refs.insert(school.reference().to_owned()));
    }
    let page = document.parsed_page();
    check!(eq; document.context_reference(), serialized_digest(&(super::super::INTERPRETATION, &capture, &page.meet, &page.skipped, &page.grade_issues, document.published_start(), document.published_end()))?);
    let frozen = serde_json::to_vec(&capture)?;
    let reopened: RetainedHtmlCaptureV1 = serde_json::from_slice(&frozen)?;
    let replay = read_retained_html_result_rows(&reopened, &rows)?;
    for index in 0..3 {
        check!(eq; available(&document.row(index).ok_or("original")?)?.reference(), available(&replay.row(index).ok_or("replayed")?)?.reference());
    }
    Ok(())
}

fn controlled(raw: &str, header: &str) -> (String, SourceRowLocator) {
    let prefix = format!("<link rel=\"canonical\" href=\"{URL}\"><script type=\"application/ld+json\">{{\"@type\":\"SportsEvent\",\"name\":\"Controlled\",\"startDate\":\"2026-05-15\",\"endDate\":\"2026-05-15\",\"sport\":\"Outdoor Track\",\"url\":\"{URL}\"}}</script><pre>Boys 110 Hurdles\r\n{header}\r\n");
    let locator = SourceRowLocator {
        ordinal: 3,
        byte_offset: prefix.len(),
        byte_length: raw.len(),
    };
    (format!("{prefix}{raw}\r\n</pre>"), locator)
}

fn header() -> String {
    format!(
        "  Pl {:<24} {:<3} {:<24}{:>9}",
        "Name", "Yr", "Team", "Time"
    )
}

fn row(name: &str, school: &str) -> String {
    format!(
        "   1 {name:<24} {:<3} {school:<24}{:>9} 8 (1)",
        "12", "13.57a"
    )
}

#[test]
fn utf8_crlf_and_repeated_school_text_use_owned_cell_not_first_text_match() -> TestResult {
    let raw = row("École Athlete", "École Athlete");
    let (body, locator) = controlled(&raw, &header());
    let capture = capture(body.as_bytes())?;
    let document = read_retained_html_result_rows(&capture, &[locator])?;
    assert_original(&document, 0)?;
    let school = available(&document.row(0).ok_or("row")?)?;
    let before = body
        .get(locator.byte_offset..school.span().start)
        .ok_or("prefix")?;
    check!(before.contains("École Athlete"));
    check!(eq; school.text(), "École Athlete");
    Ok(())
}

#[test]
fn transformed_original_rows_keep_legacy_semantics_but_withhold_school_witness() -> TestResult {
    let original = row("Runner, Controlled", "Controlled School");
    for raw in [
        original.replace("Controlled School", "Controlled&#32;School"),
        original.replace("Controlled School", "<b>Controlled School</b>"),
    ] {
        let (body, locator) = controlled(&raw, &header());
        let capture = capture(body.as_bytes())?;
        let document = read_retained_html_result_rows(&capture, &[locator])?;
        let row = document.row(0).ok_or("transformed row")?;
        check!(eq; row.parsed_row().school, "Controlled School");
        check!(matches!(
            row.school_statement()?,
            ResultSchoolStatementV1::Unavailable
        ));
        let legacy = crate::milesplit::parse_raw(&body, URL)?;
        check!(eq; serde_json::to_vec(&document.parsed_page().meet.events)?, serde_json::to_vec(&legacy.meet.events)?);
    }
    Ok(())
}

#[test]
fn duplicate_school_columns_preserve_parser_outcome_without_unique_school_witness() -> TestResult {
    let header = format!(
        "  Pl {:<24} {:<3} {:<24} {:<24}{:>9}",
        "Name", "Yr", "Team", "School", "Time"
    );
    let raw = format!(
        "   1 {:<24} {:<3} {:<24} {:<24}{:>9} 8 (1)",
        "Runner, Controlled", "12", "Controlled School", "Other School", "13.57a"
    );
    let (body, locator) = controlled(&raw, &header);
    let capture = capture(body.as_bytes())?;
    let document = read_retained_html_result_rows(&capture, &[locator])?;
    check!(matches!(
        document.row(0).ok_or("row")?.school_statement()?,
        ResultSchoolStatementV1::Unavailable
    ));
    let legacy = crate::milesplit::parse_raw(&body, URL)?;
    check!(eq; serde_json::to_vec(&document.parsed_page().meet.events)?, serde_json::to_vec(&legacy.meet.events)?);
    Ok(())
}

#[test]
fn repeated_requested_original_span_cannot_issue_school_witnesses() -> TestResult {
    let (body, locator) = controlled(&row("Runner, Controlled", "Controlled School"), &header());
    let capture = capture(body.as_bytes())?;
    check!(matches!(
        read_retained_html_result_rows(&capture, &[locator, locator]),
        Err(RetainedResultOwnershipError::Invalid(_))
    ));
    Ok(())
}
