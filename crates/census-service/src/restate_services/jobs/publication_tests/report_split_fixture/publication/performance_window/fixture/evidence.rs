use super::super::{CanonicalPerformance, TestResult};
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1, PARTICIPATION_EVIDENCE_PHASE,
};
use census_domain::model::{Evidence, SourceRef};
use census_store::{Store, Table};
use serde_json::json;

pub(super) fn append(
    store: &Store,
    first: &mut CanonicalPerformance,
    second: &mut CanonicalPerformance,
) -> TestResult {
    let schedule = schedule();
    for (performance, slug, date) in [
        (first, "synthetic-march-status", "March 27, 2026"),
        (second, "synthetic-april-status", "April 3, 2026"),
    ] {
        let url = format!("https://www.mhsaa.com/sports/boys-track-field/stories/{slug}");
        let status = format!(
            r#"<link rel="canonical" href="{url}"><article class="node__article" data-nid="123" data-path="/sports/boys-track-field/stories/{slug}"><p class="publish-date">{date}</p><h4><span class="text--secondary"><a href="https://mi.milesplit.com/athletes/14222592-synthetic-participant">Synthetic Participant</a> ♦ Detroit Catholic Central<br> Junior ♦ Track</span></h4></article>"#
        );
        let packet = IndependentParticipationPacketV1::new(
            retained(&url, &status)?,
            retained(SCHEDULE_URL, &schedule)?,
        )?;
        let reference = packet.reference()?;
        store.journal_done(
            PARTICIPATION_EVIDENCE_PHASE,
            reference.as_str(),
            &packet.payload()?,
        )?;
        let mut evidence = Evidence::parsed(
            SourceRef::new("mhsaa_synthetic_test", Some(url)),
            "2026-10-01T10:00:00Z",
        );
        evidence.note = Some(
            json!({"role": "independent_participation_v1", "packet": reference.as_str()})
                .to_string(),
        );
        performance.evidence.push(evidence);
        store.append(Table::Performances, performance)?;
    }
    Ok(())
}

const SCHEDULE_URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";

fn schedule() -> String {
    let mut body = format!(
        r#"<link rel="canonical" href="{SCHEDULE_URL}"><h2>2025-26 Track and Field Schedule</h2><ul>"#
    );
    for (game, date, name) in [
        ("1", "Mar 27 (Fri)", "Synthetic Michigan March Meet"),
        ("2", "Apr 3 (Fri)", "Synthetic Michigan April Meet"),
    ] {
        body.push_str(&format!(r#"<li class="sidearm-schedule-game" data-game-id="{game}" data-sport-id="15"><div class="sidearm-schedule-game-opponent-date"><span>{date}</span></div><div class="sidearm-schedule-game-opponent-name">{name}</div><div class="sidearm-schedule-game-location">Synthetic Michigan Venue</div></li>"#));
    }
    body.push_str("</ul>");
    body
}

fn retained(url: &str, body: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let acquired = chrono::DateTime::parse_from_rfc3339("2026-10-01T10:00:00Z")?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        acquired,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("accept".into(), "text/html".into()),
                ("accept-encoding".into(), "identity".into()),
            ],
            response_header_bytes:
                b"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\r\n".to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: acquired,
        },
    )?)
}
