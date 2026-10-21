use super::super::{RetainedHtmlCaptureV1, SourceRowLocator};
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};

pub(super) type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(super) const BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));
const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";

pub(super) fn locators() -> [SourceRowLocator; 2] {
    [
        SourceRowLocator {
            ordinal: 118,
            byte_offset: 205297,
            byte_length: 1409,
        },
        SourceRowLocator {
            ordinal: 119,
            byte_offset: 206710,
            byte_length: 1360,
        },
    ]
}

pub(super) fn capture(body: &[u8]) -> TestResult<RetainedHtmlCaptureV1> {
    let completed = DateTime::parse_from_rfc3339("2026-09-22T03:55:31Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        completed,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "text/html".into())],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=utf-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-09-22T03:55:30Z")?.with_timezone(&Utc),
        },
    )?)
}

pub(super) fn changed_row(
    change: impl FnOnce(&str) -> String,
) -> TestResult<(RetainedHtmlCaptureV1, SourceRowLocator)> {
    let mut body = std::str::from_utf8(BODY)?.to_owned();
    let mut locator = locators()[0];
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("fixture row overflow")?;
    let replacement = change(
        body.get(locator.byte_offset..end)
            .ok_or("fixture original row")?,
    );
    locator.byte_length = replacement.len();
    body.replace_range(locator.byte_offset..end, &replacement);
    Ok((capture(body.as_bytes())?, locator))
}

pub(super) fn first_replaced(
    from: &str,
    to: &str,
) -> TestResult<(RetainedHtmlCaptureV1, SourceRowLocator)> {
    changed_row(|row| row.replacen(from, to, 1))
}

pub(super) fn prefix_replaced(
    from: &str,
    to: &str,
) -> TestResult<(RetainedHtmlCaptureV1, SourceRowLocator)> {
    let mut body = std::str::from_utf8(BODY)?.to_owned();
    let mut row = locators()[0];
    let start = body.find(from).ok_or("fixture topology prefix")?;
    let end = start.checked_add(from.len()).ok_or("fixture prefix end")?;
    if end > row.byte_offset {
        return Err("fixture prefix follows selected row".into());
    }
    let added = to
        .len()
        .checked_sub(from.len())
        .ok_or("fixture prefix shrink")?;
    row.byte_offset = row
        .byte_offset
        .checked_add(added)
        .ok_or("fixture prefix offset")?;
    body.replace_range(start..end, to);
    Ok((capture(body.as_bytes())?, row))
}
