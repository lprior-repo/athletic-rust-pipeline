use super::*;

fn capture(url: &str, body: &[u8], headers: &[u8]) -> TestResult<RetainedHtmlCaptureV1> {
    let time = DateTime::parse_from_rfc3339("2026-10-16T00:10:00Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        time,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "*/*".into())],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: time,
        },
    )?)
}

pub(super) fn packet() -> TestResult<IndependentParticipationPacketV1> {
    let status = capture(
        "https://www.mhsaa.com/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks",
        include_bytes!("../../../tests/fixtures/mhsaa-status.html"),
        include_bytes!("../../../tests/fixtures/mhsaa-status.headers"),
    )?;
    let schedule = capture(
        "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26",
        include_bytes!("../../../tests/fixtures/dcc-schedule.html"),
        include_bytes!("../../../tests/fixtures/dcc-schedule.headers"),
    )?;
    Ok(IndependentParticipationPacketV1::new(status, schedule)?)
}

pub(super) fn result() -> TestResult<RetainedHtmlCaptureV1> {
    capture(
        "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw",
        include_bytes!("../../../result_ownership/tests/fixtures/milesplit-result.html"),
        include_bytes!("../../../result_ownership/tests/fixtures/milesplit-result.headers"),
    )
}
