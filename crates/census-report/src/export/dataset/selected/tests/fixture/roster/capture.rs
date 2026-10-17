use super::{hash, FetchOutcome, TeamRef, TestResult};
use census_crawl::milesplit::{parse_captured_roster, RosterOutcome, SourceRowLocator};
use serde_json::json;

pub(crate) fn parse(team: &TeamRef, body: Vec<u8>, fetched_at: &str) -> TestResult<RosterOutcome> {
    let capture = FetchOutcome {
        url: format!("{}/roster", team.url),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: hash(&body),
        bytes: body.len(),
        fetched_at: fetched_at.into(),
        from_cache: false,
        content_type: Some("text/html".into()),
        body,
    };
    let verdict =
        parse_captured_roster(std::str::from_utf8(&capture.body)?, team.clone(), &capture)?;
    Ok(RosterOutcome { capture, verdict })
}

pub(crate) fn row_metadata(
    capture: &FetchOutcome,
    team: &TeamRef,
    athlete_id: &str,
    row: SourceRowLocator,
) -> TestResult<String> {
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("roster row overflow")?;
    let span = capture
        .body
        .get(row.byte_offset..end)
        .ok_or("roster row bytes")?;
    let key = format!(
        "{}#capture={}&row={}&offset={}&length={}",
        capture.url, capture.content_digest, row.ordinal, row.byte_offset, row.byte_length
    );
    Ok(json!({
        "role": "roster_positive_row_metadata_v1", "source_row_key": key,
        "provider_team_id": team.id, "provider_athlete_id": athlete_id,
        "capture_url": capture.url, "response_url": null,
        "capture_sha256": capture.content_digest, "capture_bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "row": row, "row_sha256": hash(span),
    }).to_string())
}
