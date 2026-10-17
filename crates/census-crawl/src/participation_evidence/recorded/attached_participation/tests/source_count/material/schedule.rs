use super::*;

const URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";
const BODY: &str = r#"<!doctype html><html><head>
<link rel="canonical" href="https://dccshamrocks.net/sports/track-and-field/schedule/2025-26">
<meta name="controlled-test-fixture" content="synthetic-count-schedule-v1">
</head><body><p>SYNTHETIC CONTROLLED SCHEDULE. Not a fetched official page or independent genuine evidence.</p>
<h2>2025-26 Track and Field Schedule</h2><ul>
<li class="sidearm-schedule-game" data-game-id="990001" data-sport-id="15">
<div class="sidearm-schedule-game-opponent-date"><span>May 15 (Fri)</span></div>
<div class="sidearm-schedule-game-opponent-name">MHSAA Regionals</div>
<div class="sidearm-schedule-game-location">Milford HS</div>
</li></ul></body></html>"#;

pub(super) fn controlled(original: &RetainedHtmlCaptureV1) -> TestResult<RetainedHtmlCaptureV1> {
    check!(eq; original.requested_url(), URL);
    check!(eq; original.body_sha256(), "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb");
    let original_bytes = serde_json::to_vec(original)?;
    check!(BODY.len() <= 2048);
    let at = DateTime::parse_from_rfc3339("2026-10-16T00:10:00Z")?.with_timezone(&Utc);
    let capture = RetainedHtmlCaptureV1::new(
        URL, URL, 200, at, BODY.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("X-Controlled-Test-Fixture".into(), "synthetic-count-schedule-v1".into())],
            response_header_bytes: b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nX-Synthetic-Test: source-count-schedule-v1\r\n\r\n".to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: at,
        },
    )?;
    check!(ne; capture.body_sha256(), original.body_sha256());
    check!(ne; serde_json::to_vec(&capture)?, original_bytes);
    let day = census_domain::model::ProfileDate::parse("2026-05-15")?;
    let parsed = crate::participation_evidence::schedule::read(&capture, &day)?;
    check!(eq; parsed.school_year.get(), 2025);
    check!(eq; parsed.competition, "MHSAA Regionals");
    check!(eq; parsed.venue, "Milford HS");
    check!(eq; parsed.scope, format!("{URL}#game-990001"));
    check!(eq; serde_json::to_vec(original)?, original_bytes);
    Ok(capture)
}
