use super::super::super::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use super::*;
use chrono::{DateTime, Utc};

pub(super) const BODY: &[u8] =
    include_bytes!("../../result_ownership/tests/fixtures/milesplit-result.html");
pub(super) const HEADERS: &[u8] =
    include_bytes!("../../result_ownership/tests/fixtures/milesplit-result.headers");
pub(super) const URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
pub(super) const ATTACHMENT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const SCHOOL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(super) fn locators() -> [SourceRowLocator; 3] {
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

pub(super) fn capture() -> TestResult<RetainedHtmlCaptureV1> {
    capture_with(BODY, "OpenAI-research/0.1")
}

pub(super) fn capture_with(body: &[u8], user_agent: &str) -> TestResult<RetainedHtmlCaptureV1> {
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
                ("User-Agent".into(), user_agent.into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: HEADERS.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-10-16T00:06:38Z")?.with_timezone(&Utc),
        },
    )?)
}

pub(super) fn subjects(document: &CheckedRetainedResultDocumentV1<'_>) -> TestResult<Vec<String>> {
    (0..document.selected_count())
        .map(|index| {
            Ok(document
                .row(index)
                .ok_or("source row")?
                .subject()?
                .reference())
        })
        .collect()
}

pub(super) fn targets<'a>(
    subjects: &'a [String],
    athlete: &'a AthleteId,
    school: &'a SchoolId,
) -> Vec<ProposedHtmlResultTargetV1<'a>> {
    subjects
        .iter()
        .zip(locators())
        .map(|(subject, locator)| ProposedHtmlResultTargetV1 {
            locator,
            subject_reference: subject,
            proposed_athlete: athlete,
            proposed_school: school,
            proposed_attachment_reference: ATTACHMENT,
            proposed_school_binding_reference: SCHOOL,
        })
        .collect()
}

pub(super) fn athlete() -> AthleteId {
    AthleteId::mint("ath_subject", &["unadmitted projector fixture"])
}
pub(super) fn school() -> SchoolId {
    SchoolId::mint("sch", &["unadmitted projector fixture"])
}
