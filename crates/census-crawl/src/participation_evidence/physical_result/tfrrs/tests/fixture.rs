use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};

pub(super) const BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));
pub(super) const ATTACHMENT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const SCHOOL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

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
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339("2026-09-22T03:55:31Z")?.with_timezone(&Utc),
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

pub(super) fn athlete() -> AthleteId {
    AthleteId::mint("ath_subject", &["unadmitted TFRRS projection"])
}
pub(super) fn school() -> SchoolId {
    SchoolId::mint("sch", &["unadmitted TFRRS projection"])
}
pub(super) fn target<'a>(
    locator: SourceRowLocator,
    subject: &'a str,
    athlete: &'a AthleteId,
    school: &'a SchoolId,
) -> ProposedHtmlResultTargetV1<'a> {
    ProposedHtmlResultTargetV1 {
        locator,
        subject_reference: subject,
        proposed_athlete: athlete,
        proposed_school: school,
        proposed_attachment_reference: ATTACHMENT,
        proposed_school_binding_reference: SCHOOL,
    }
}

pub(super) fn altered_event(label: &str) -> TestResult<(RetainedHtmlCaptureV1, SourceRowLocator)> {
    let original = std::str::from_utf8(BODY)?;
    let mut locator = locators()[0];
    let row_end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("fixture end")?;
    let prefix = original
        .get(..locator.byte_offset)
        .ok_or("fixture prefix")?;
    let header = prefix
        .rfind("3200 Meters\n")
        .ok_or("fixture source event")?;
    let header_end = header
        .checked_add("3200 Meters".len())
        .ok_or("fixture header")?;
    let mut changed_prefix = prefix.to_owned();
    changed_prefix.replace_range(header..header_end, label);
    let row = original
        .get(locator.byte_offset..row_end)
        .ok_or("fixture row")?;
    let slug = label.split_whitespace().collect::<Vec<_>>().join("-");
    let changed_row = row.replacen("/3200-Meters\"", &format!("/{slug}\""), 1);
    let mut changed = changed_prefix;
    locator.byte_offset = changed.len();
    locator.byte_length = changed_row.len();
    changed.push_str(&changed_row);
    changed.push_str(original.get(row_end..).ok_or("fixture suffix")?);
    Ok((capture(changed.as_bytes())?, locator))
}
