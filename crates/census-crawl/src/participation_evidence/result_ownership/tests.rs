use super::super::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use super::*;
use census_domain::model::{EventKind, ExactSeconds, Mark, TimingMethod};
use chrono::{DateTime, Utc};

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const BODY: &[u8] = include_bytes!("tests/fixtures/milesplit-result.html");
const HEADERS: &[u8] = include_bytes!("tests/fixtures/milesplit-result.headers");

fn locators() -> [SourceRowLocator; 3] {
    [
        SourceRowLocator {
            ordinal: 527,
            byte_offset: 80520,
            byte_length: 100,
        },
        SourceRowLocator {
            ordinal: 540,
            byte_offset: 81648,
            byte_length: 92,
        },
        SourceRowLocator {
            ordinal: 572,
            byte_offset: 84465,
            byte_length: 92,
        },
    ]
}

fn capture(body: &[u8]) -> TestResult<RetainedHtmlCaptureV1> {
    let completed = DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        completed,
        body,
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

#[test]
fn genuine_original_rows_have_document_ownership_without_a_provider_person_id() -> TestResult {
    let capture = capture(BODY)?;
    check!(eq;
        capture.body_sha256(),
        "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5"
    );
    check!(eq;
        super::super::hash(HEADERS),
        "15dfbd070a93ae2e3fd19cd2beeb87d4f189861e0266f2a2fdf7ce01ce7a3ef8"
    );
    let document = read_retained_html_result_rows(&capture, &locators())?;
    check!(eq; document.original_document().as_bytes(), BODY);
    check!(eq; document.meet_id(), "763888");
    check!(eq; document.result_document_id(), "1304896");
    check!(eq; document.published_start(), "2026-05-15");
    check!(eq; document.published_end(), Some("2026-05-15"));
    check!(eq; document.selected_count(), 3);
    check!(!document.original_document().contains("7223777"));
    check!(!document.original_document().contains("12439"));
    for (index, mark) in ["13.57", "14.10", "36.23"].iter().enumerate() {
        let row = document.row(index).ok_or("checked row")?;
        check!(eq; row.locator(), locators()[index]);
        check!(eq; row.parsed_row().name, "Zacchaeus Brocks");
        check!(eq; row.parsed_row().school, "Detroit Catholic Central");
        check!(eq; row.parsed_row().grade.map(|grade| grade.get()), Some(12));
        check!(eq;
            row.parsed_row().mark,
            Mark::TimeSeconds(ExactSeconds::parse(mark)?)
        );
        check!(eq; row.parsed_row().timing, Some(TimingMethod::Fat));
        check!(eq;
            row.event().kind,
            if index == 2 {
                EventKind::Track300mHurdles
            } else {
                EventKind::Track110mHurdles
            }
        );
        check!(row.original_row()?.contains(&format!("{mark}a")));
        let subject = row.subject()?;
        check!(eq; subject.document_namespace(), "milesplit_result_document_v1");
        check!(eq; subject.native_document_id(), "1304896");
        check!(eq; subject.document_url(), URL);
    }
    Ok(())
}

#[test]
fn frozen_capture_reparse_preserves_context_and_exact_subject_references() -> TestResult {
    let original = capture(BODY)?;
    let frozen = serde_json::to_vec(&original)?;
    let reopened: RetainedHtmlCaptureV1 = serde_json::from_slice(&frozen)?;
    let before = read_retained_html_result_rows(&original, &locators())?;
    let after = read_retained_html_result_rows(&reopened, &locators())?;
    check!(eq; before.context_reference(), after.context_reference());
    check!(eq; before.parsed_page(), after.parsed_page());
    for index in 0..3 {
        let before = before.row(index).ok_or("before row")?;
        let after = after.row(index).ok_or("after row")?;
        check!(eq; before.reference(), after.reference());
        check!(eq; before.original_row_digest(), after.original_row_digest());
        check!(eq; before.subject()?, after.subject()?);
    }
    Ok(())
}

#[test]
fn late_missing_requested_row_discards_earlier_provisional_successes() -> TestResult {
    let capture = capture(BODY)?;
    let mut requested = locators();
    requested[2].byte_length -= 1;
    check!(matches!(
        read_retained_html_result_rows(&capture, &requested),
        Err(RetainedResultOwnershipError::Invalid(
            "requested row is not an exact successful original row"
        ))
    ));
    check!(eq;
        read_retained_html_result_rows(&capture, &locators())?.selected_count(),
        3
    );
    Ok(())
}

#[test]
fn repeated_reordered_and_one_over_locator_requests_fail_closed() -> TestResult {
    let capture = capture(BODY)?;
    check!(read_retained_html_result_rows(&capture, &[locators()[0], locators()[0]]).is_err());
    check!(read_retained_html_result_rows(&capture, &[locators()[1], locators()[0]]).is_err());
    check!(selection::admit_count(64).is_ok());
    check!(matches!(
        read_retained_html_result_rows(&capture, &[locators()[0]; 65]),
        Err(RetainedResultOwnershipError::Capacity {
            needed: 65,
            available: 64
        })
    ));
    Ok(())
}

#[test]
fn altered_capture_digest_and_foreign_published_owner_refuse() -> TestResult {
    let capture = capture(BODY)?;
    let mut wire = serde_json::to_value(&capture)?;
    wire["body_sha256"] = serde_json::Value::String("0".repeat(64));
    let corrupt: RetainedHtmlCaptureV1 = serde_json::from_value(wire)?;
    check!(matches!(
        read_retained_html_result_rows(&corrupt, &locators()),
        Err(RetainedResultOwnershipError::Capture(_))
    ));
    let text = std::str::from_utf8(BODY)?.replace("/results/1304896/raw", "/results/1304897/raw");
    let foreign = self::capture(text.as_bytes())?;
    check!(matches!(
        read_retained_html_result_rows(&foreign, &locators()),
        Err(RetainedResultOwnershipError::Parser(_))
    ));
    Ok(())
}

#[test]
fn inert_competing_comment_and_ambiguous_pre_blocks_cannot_own_rows() -> TestResult {
    for body in [
        "<template><pre>row</pre></template>",
        "<pre>first</pre><pre>second</pre>",
        "<!-- <pre>inert</pre> --><pre>active</pre>",
        "<pre hidden>row</pre>",
        "<script>const value = '<pre>inert</pre>';</script><pre>active</pre>",
        "<pre data-x='>'>row</pre>",
        "<div hidden><pre>row</pre></div>",
        "<div inert><pre>row</pre></div>",
        "<pre>row</pre><div hidden>late hidden sibling</div>",
        "<div inert></div><pre>row</pre>",
    ] {
        check!(active::owned_pre(body).is_err(), "{body}");
    }
    check!(eq; active::owned_pre("<pre>one\nrow</pre>")?, 5..12);
    Ok(())
}
