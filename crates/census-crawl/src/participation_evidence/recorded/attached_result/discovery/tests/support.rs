use super::*;

pub(super) const URL: &str = "https://mi.milesplit.com/meets/123/results/456/raw";

pub(super) struct Document {
    pub capture: RetainedHtmlCaptureV1,
    pub html: String,
    pub first: String,
    pub last: String,
    pub rejected: String,
}

pub(super) fn document() -> TestResult<Document> {
    let header = format!(
        "  Pl {:<24} {:<3} {:<24}{:>9}{:>8}",
        "Name", "Yr", "Team", "Time", "Wind"
    );
    let row = |name: &str, wind: &str| {
        format!(
            "   1 {name:<24} {:<3} {:<24}{:>9}{wind:>8}",
            "12", "Synthetic School", "13.26"
        )
    };
    let first = row("MÜLLER, Émile", "+0.2");
    let rejected = row("Rejected Runner", "NaN");
    let last = row("Last Runner", "-0.4");
    let block = format!(
        "Boys 110 Hurdles Finals\r\n{header}\r\n{first}\r\n\r\n{rejected}\r\n\r\n{last}\r\n"
    );
    let html = format!("<link rel=\"canonical\" href=\"{URL}\"><script type=\"application/ld+json\">{{\"@type\":\"SportsEvent\",\"name\":\"Synthetic Meet\",\"startDate\":\"2026-05-15\",\"sport\":\"Outdoor Track\",\"url\":\"{URL}\"}}</script><pre>{block}</pre>");
    let capture = capture(html.as_bytes())?;
    Ok(Document {
        capture,
        html,
        first,
        last,
        rejected,
    })
}

pub(super) fn capture(body: &[u8]) -> TestResult<RetainedHtmlCaptureV1> {
    let at = DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        at,
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: Vec::new(),
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: at,
        },
    )?)
}

pub(super) fn span(html: &str, row: &str) -> TestResult<OriginalResultSpan> {
    let start = html.find(row).ok_or("original row absent")?;
    let end = start
        .checked_add(row.len())
        .ok_or("original row end overflow")?;
    Ok(
        OriginalResultSpan::parse(format!("html_bytes:{start}:{end}").as_bytes())
            .ok_or("canonical span rejected")?,
    )
}

pub(super) fn locator(html: &str, row: &str, ordinal: u32) -> TestResult<SourceRowLocator> {
    Ok(SourceRowLocator {
        ordinal,
        byte_offset: html.find(row).ok_or("original row absent")?,
        byte_length: row.len(),
    })
}
