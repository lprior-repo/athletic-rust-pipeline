use census_crawl::milesplit::Site;
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1, PARTICIPATION_EVIDENCE_PHASE,
};
use census_domain::model::{CanonicalPerformance, CanonicalSchool, Evidence, SourceRef};
use census_store::{Store, Table};
use serde_json::json;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

const SCHEDULE_URL: &str =
    "https://synthetic-selected-high.test/sports/track-and-field/schedule/2025-26";
const ACQUIRED_AT: &str = "2026-09-20T10:00:00Z";

pub(super) fn append(store: &Store, performance: &mut CanonicalPerformance) -> TestResult<()> {
    let athlete = store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.id == performance.athlete)
        .ok_or("selected fixture evidence athlete")?;
    let school = store
        .snapshot()
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| school.id == athlete.school)
        .ok_or("selected fixture evidence school")?;
    let person = athlete
        .source
        .as_ref()
        .ok_or("selected fixture provider athlete")?;
    let site = Site::for_jurisdiction(school.state.ok_or("selected fixture jurisdiction")?);
    let profile = super::athlete_profile_url(&site.host(), &person.id, &athlete.canonical_name);
    let slug = "synthetic-selected-status";
    let url = format!("https://www.mhsaa.com/sports/girls-track-field/stories/{slug}");
    let status = format!(
        r#"<link rel="canonical" href="{url}"><article class="node__article" data-nid="123" data-path="/sports/girls-track-field/stories/{slug}"><p class="publish-date">April 17, 2026</p><h4><span class="text--secondary"><a href="{profile}">{}</a> ♦ {}<br> Junior ♦ Track</span></h4></article>"#,
        athlete.canonical_name, school.name
    );
    let acquired = chrono::DateTime::parse_from_rfc3339(ACQUIRED_AT)?.to_utc();
    let packet = IndependentParticipationPacketV1::new(
        RetainedHtmlCaptureV1::new(
            &url,
            &url,
            200,
            acquired,
            status.as_bytes(),
            transport(acquired),
        )?,
        RetainedHtmlCaptureV1::new(
            SCHEDULE_URL,
            SCHEDULE_URL,
            200,
            acquired,
            schedule(performance.date.as_str())?.as_bytes(),
            transport(acquired),
        )?,
    )?;
    let reference = packet.reference()?;
    let payload = packet.payload()?;
    store.journal_done(PARTICIPATION_EVIDENCE_PHASE, reference.as_str(), &payload)?;
    let mut evidence = Evidence::parsed(
        SourceRef::new("mhsaa_synthetic_test", Some(url)),
        ACQUIRED_AT,
    );
    evidence.note = Some(
        json!({"role": "independent_participation_v1", "packet": reference.as_str()}).to_string(),
    );
    performance.evidence.push(evidence);
    store.append(Table::Performances, performance)?;
    Ok(())
}

fn schedule(performance_date: &str) -> TestResult<String> {
    let (game, date) = match performance_date {
        "2026-04-17" => ("1", "Apr 17 (Fri)"),
        _ => return Err("selected fixture schedule covers the April captured meet".into()),
    };
    let mut body = format!(
        r#"<link rel="canonical" href="{SCHEDULE_URL}"><h2>2025-26 Track and Field Schedule</h2><ul>"#
    );
    body.push_str(&format!(r#"<li class="sidearm-schedule-game" data-game-id="{game}" data-sport-id="15"><div class="sidearm-schedule-game-opponent-date"><span>{date}</span></div><div class="sidearm-schedule-game-opponent-name">Synthetic Michigan Girls Meet</div><div class="sidearm-schedule-game-location">Synthetic Michigan Venue</div></li>"#));
    body.push_str("</ul>");
    Ok(body)
}

fn transport(acquired: chrono::DateTime<chrono::Utc>) -> RetainedHttpProvenanceV1 {
    RetainedHttpProvenanceV1 {
        request_method: "GET".into(),
        request_headers: vec![
            ("accept".into(), "text/html".into()),
            ("accept-encoding".into(), "identity".into()),
        ],
        response_header_bytes: b"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\n\r\n"
            .to_vec(),
        content_type: "text/html; charset=utf-8".into(),
        body_encoding: RetainedBodyEncodingV1::Identity,
        started_at: acquired,
    }
}
