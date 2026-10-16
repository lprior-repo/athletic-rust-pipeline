use super::*;
use chrono::{DateTime, Utc};

pub(super) const PROFILE: &[u8] =
    include_bytes!("../../../provider_target/tests/fixtures/profile.html");
pub(super) const RESULTS: &[u8] =
    include_bytes!("../../../provider_target/tests/fixtures/linked-results.html");
pub(super) const HTML: &[u8] =
    include_bytes!("../../../result_ownership/tests/fixtures/milesplit-result.html");
pub(super) const STATUS: &[u8] = include_bytes!("../../../tests/fixtures/mhsaa-status.html");
pub(super) const SCHEDULE: &[u8] = include_bytes!("../../../tests/fixtures/dcc-schedule.html");
const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks";
const RESULTS_URL: &str =
    "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439";
const HTML_URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const STATUS_URL: &str = "https://www.mhsaa.com/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks";
const SCHEDULE_URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";

fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    start: &str,
    end: &str,
    content_type: &str,
    accept: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(end)?.with_timezone(&Utc),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), accept.into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: content_type.into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(start)?.with_timezone(&Utc),
        },
    )?)
}

pub(super) fn provider_packet(results: &[u8]) -> TestResult<SourceTargetPacketV1> {
    Ok(SourceTargetPacketV1::new(
        capture(
            PROFILE_URL,
            PROFILE,
            include_bytes!("../../../provider_target/tests/fixtures/profile.headers"),
            "2026-10-16T00:50:21Z",
            "2026-10-16T00:50:27Z",
            "text/html; charset=UTF-8",
            "text/html",
        )?,
        capture(
            RESULTS_URL,
            results,
            include_bytes!("../../../provider_target/tests/fixtures/linked-results.headers"),
            "2026-10-16T00:49:13Z",
            "2026-10-16T00:49:19Z",
            "text/html; charset=UTF-8",
            "text/html",
        )?,
    )?)
}

pub(super) fn html_capture() -> TestResult<RetainedHtmlCaptureV1> {
    capture(
        HTML_URL,
        HTML,
        include_bytes!("../../../result_ownership/tests/fixtures/milesplit-result.headers"),
        "2026-10-16T00:06:38Z",
        "2026-10-16T00:06:57Z",
        "text/html; charset=UTF-8",
        "*/*",
    )
}

pub(super) fn participation_packet() -> TestResult<IndependentParticipationPacketV1> {
    Ok(IndependentParticipationPacketV1::new(
        capture(
            STATUS_URL,
            STATUS,
            include_bytes!("../../../tests/fixtures/mhsaa-status.headers"),
            "2026-10-16T00:05:12Z",
            "2026-10-16T00:05:15Z",
            "text/html; charset=utf-8",
            "*/*",
        )?,
        capture(
            SCHEDULE_URL,
            SCHEDULE,
            include_bytes!("../../../tests/fixtures/dcc-schedule.headers"),
            "2026-10-16T00:05:51Z",
            "2026-10-16T00:05:56Z",
            "text/html; charset=utf-8",
            "*/*",
        )?,
    )?)
}

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
