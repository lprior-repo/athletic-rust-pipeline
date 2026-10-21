use super::TestResult;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use chrono::{DateTime, Utc};

pub(super) const BODY: &str = "<html>controlled transport metadata parity</html>";
pub(super) const ROSTER_URL: &str =
    "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";

pub(super) fn packet() -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture("https://mi.milesplit.com/teams")?,
        capture(ROSTER_URL)?,
    )?)
}

fn capture(url: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let instant = DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        instant,
        BODY.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "text/html".into())],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: instant,
        },
    )?)
}
