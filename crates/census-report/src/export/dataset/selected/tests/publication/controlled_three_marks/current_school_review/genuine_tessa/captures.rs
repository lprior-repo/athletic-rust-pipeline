use super::*;

const DIRECTORY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/recorded/attached_result/discovery/tests/tfrrs/genuine_tessa/fixture/directory.html"));
pub(super) const ROSTER: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/recorded/attached_result/discovery/tests/tfrrs/genuine_tessa/fixture/roster.html"));

pub(super) fn school_packet() -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture(
            "https://in.milesplit.com/teams",
            DIRECTORY,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/recorded/attached_result/discovery/tests/tfrrs/genuine_tessa/fixture/directory.headers")),
            "2026-10-21T21:55:10.904913+00:00",
            "2026-10-21T21:55:11.360682+00:00",
        )?,
        capture(
            "https://in.milesplit.com/teams/10561-roncalli-high-school/roster",
            ROSTER,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/recorded/attached_result/discovery/tests/tfrrs/genuine_tessa/fixture/roster.headers")),
            "2026-10-21T21:55:11.360968+00:00",
            "2026-10-21T21:55:11.986137+00:00",
        )?,
    )?)
}

pub(super) fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    start: &str,
    end: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(end)?.to_utc(),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
                ("Accept-Encoding".into(), "identity".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(start)?.to_utc(),
        },
    )?)
}

pub(super) fn tfrrs() -> TestResult<RetainedHtmlCaptureV1> {
    capture(
        "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i",
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html")),
        b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: retained-genuine-TFRRS-body\r\n\r\n",
        "2026-10-21T12:00:00Z", "2026-10-21T12:00:01Z",
    )
}
