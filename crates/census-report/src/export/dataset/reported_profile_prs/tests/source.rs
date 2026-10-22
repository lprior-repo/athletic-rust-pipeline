use super::TestResult;
use census_crawl::participation_evidence::provider_target::SourceTargetPacketV1;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use chrono::{DateTime, TimeDelta, Utc};

const PROFILE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_target/tests/fixtures/profile.html"
));
const RESULTS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_target/tests/fixtures/linked-results.html"));
const PROFILE_HEADERS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_target/tests/fixtures/profile.headers"
));
const RESULT_HEADERS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_target/tests/fixtures/linked-results.headers"));

pub(crate) fn packet(id: &str, acquired: &str) -> TestResult<SourceTargetPacketV1> {
    packet_with_profile(id, acquired, Ok)
}

pub(super) fn packet_with_profile(
    id: &str,
    acquired: &str,
    transform: impl FnOnce(String) -> TestResult<String>,
) -> TestResult<SourceTargetPacketV1> {
    let captured = DateTime::parse_from_rfc3339(acquired)?.with_timezone(&Utc);
    let profile_url = format!("https://mi.milesplit.com/athletes/{id}-zacchaeus-brocks");
    let profile = transform(std::str::from_utf8(PROFILE)?.replace("7223777", id))?;
    let results = std::str::from_utf8(RESULTS)?.replace("7223777", id);
    let linked = captured
        .checked_sub_signed(TimeDelta::seconds(68))
        .ok_or("capture time")?;
    let packet = SourceTargetPacketV1::new(
        capture(&profile_url, profile.as_bytes(), PROFILE_HEADERS, captured)?,
        capture(
            "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439",
            results.as_bytes(),
            RESULT_HEADERS,
            linked,
        )?,
    )?;
    packet.read()?;
    Ok(packet)
}

fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    acquired: DateTime<Utc>,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        acquired,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: acquired
                .checked_sub_signed(TimeDelta::seconds(6))
                .ok_or("start time")?,
        },
    )?)
}
