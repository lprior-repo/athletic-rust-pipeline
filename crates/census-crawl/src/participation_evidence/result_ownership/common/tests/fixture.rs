use super::{CheckedHtmlResultSourceV1, RetainedHtmlCaptureV1, SourceRowLocator, TestResult};
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};

const MS_URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const TFRRS_URL: &str =
    "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const MS_BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"
));
const MS_HEADERS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.headers"
));
const TFRRS_BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));

pub(in crate::participation_evidence) fn locators(
    source: CheckedHtmlResultSourceV1,
) -> Vec<SourceRowLocator> {
    let values = match source {
        CheckedHtmlResultSourceV1::MileSplit => {
            vec![(527, 80520, 100), (540, 81648, 92), (572, 84465, 92)]
        }
        CheckedHtmlResultSourceV1::Tfrrs => vec![(118, 205297, 1409), (119, 206710, 1360)],
    };
    values
        .into_iter()
        .map(|(ordinal, byte_offset, byte_length)| SourceRowLocator {
            ordinal,
            byte_offset,
            byte_length,
        })
        .collect()
}

pub(in crate::participation_evidence) fn capture(
    source: CheckedHtmlResultSourceV1,
) -> TestResult<RetainedHtmlCaptureV1> {
    match source {
        CheckedHtmlResultSourceV1::MileSplit => milesplit_original_transport(),
        CheckedHtmlResultSourceV1::Tfrrs => tfrrs_genuine_body_with_controlled_transport(),
    }
}

fn milesplit_original_transport() -> TestResult<RetainedHtmlCaptureV1> {
    let acquired = DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        MS_URL,
        MS_URL,
        200,
        acquired,
        MS_BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: MS_HEADERS.into(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-10-16T00:06:38Z")?.with_timezone(&Utc),
        },
    )?)
}

fn tfrrs_genuine_body_with_controlled_transport() -> TestResult<RetainedHtmlCaptureV1> {
    let acquired = DateTime::parse_from_rfc3339("2026-09-22T03:55:31Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        TFRRS_URL,
        TFRRS_URL,
        200,
        acquired,
        TFRRS_BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "text/html".into())],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=utf-8\r\n\r\n"
                .into(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-09-22T03:55:30Z")?.with_timezone(&Utc),
        },
    )?)
}
