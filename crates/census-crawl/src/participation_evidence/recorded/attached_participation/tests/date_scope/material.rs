use super::*;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use chrono::{DateTime, Utc};

pub(super) fn packet(
    index: usize,
) -> TestResult<crate::participation_evidence::IndependentParticipationPacketV1> {
    check!((1..=2).contains(&index));
    let day = if index == 1 {
        "May 15, 2026"
    } else {
        "May 22, 2026"
    };
    let url = format!(
        "https://www.mhsaa.com/sports/boys-track-field/stories/synthetic-date-scope-fixture-{index}"
    );
    let path = format!("/sports/boys-track-field/stories/synthetic-date-scope-fixture-{index}");
    let body = format!(
        "<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body><article class=\"node__article\" data-path=\"{path}\" data-nid=\"9910{index}\"><p class=\"publish-date\">{day}</p><h4><span class=\"text--secondary\">Zacchaeus Brocks ♦ Detroit Catholic Central | Senior ♦ Track</span></h4><p>SYNTHETIC DATE SCOPE TEST DOCUMENT {index}. No real publication or additional person authority is claimed.</p></article></body></html>"
    );
    let time = DateTime::parse_from_rfc3339("2026-10-16T00:10:00Z")?.with_timezone(&Utc);
    let status = RetainedHtmlCaptureV1::new(
        &url,
        &url,
        200,
        time,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "*/*".into())],
            response_header_bytes: b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nX-Synthetic-Test: date-scope\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: time,
        },
    )?;
    let schedule_url = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";
    let schedule = RetainedHtmlCaptureV1::new(
        schedule_url,
        schedule_url,
        200,
        time,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/participation_evidence/tests/fixtures/dcc-schedule.html"
        )),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "*/*".into())],
            response_header_bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/participation_evidence/tests/fixtures/dcc-schedule.headers"
            ))
            .to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: time,
        },
    )?;
    Ok(crate::participation_evidence::IndependentParticipationPacketV1::new(status, schedule)?)
}
