use super::super::{CanonicalPerformance, TestResult};
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1, PARTICIPATION_EVIDENCE_PHASE,
};
use census_domain::model::{Evidence, SourceRef};
use census_store::{Store, Table};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod oracle;

#[derive(Clone)]
pub(in super::super) struct ExpectedProofs {
    pub eligible: Vec<String>,
    pub excluded: Vec<String>,
    pub packets: BTreeMap<String, Value>,
}

pub(super) fn append(
    store: &Store,
    first: &mut CanonicalPerformance,
    second: &mut CanonicalPerformance,
) -> TestResult<ExpectedProofs> {
    let schedule = schedule();
    let mut packets = BTreeMap::new();
    let mut refs = Vec::new();
    for (performance, slug, date, game) in [
        (&mut *first, "synthetic-march-status", "March 27, 2026", "1"),
        (&mut *second, "synthetic-april-status", "April 3, 2026", "2"),
    ] {
        let url = format!("https://www.mhsaa.com/sports/boys-track-field/stories/{slug}");
        let status = format!(
            r#"<link rel="canonical" href="{url}"><article class="node__article" data-nid="123" data-path="/sports/boys-track-field/stories/{slug}"><p class="publish-date">{date}</p><h4><span class="text--secondary"><a href="https://mi.milesplit.com/athletes/14222592-synthetic-participant">Synthetic Participant</a> ♦ Detroit Catholic Central<br> Junior ♦ Track</span></h4></article>"#
        );
        let acquired = chrono::DateTime::parse_from_rfc3339("2026-10-01T10:00:00Z")?.to_utc();
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
                schedule.as_bytes(),
                transport(acquired),
            )?,
        )?;
        let reference = packet.reference()?;
        let payload = packet.payload()?;
        store.journal_done(PARTICIPATION_EVIDENCE_PHASE, reference.as_str(), &payload)?;
        packets.insert(reference.as_str().to_owned(), payload);
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
        let mut expected = BTreeSet::from([reference.as_str().to_owned()]);
        proof(&mut expected, &status, "<h4>", "</h4>")?;
        proof(
            &mut expected,
            &schedule,
            &format!("<li class=\"sidearm-schedule-game\" data-game-id=\"{game}\""),
            "</li>",
        )?;
        refs.push(expected);
    }
    let [first_refs, second_refs]: [BTreeSet<String>; 2] = refs
        .try_into()
        .map_err(|_| "two exact-day packet proof sets")?;
    Ok(ExpectedProofs {
        eligible: oracle::references(store, first, first_refs)?,
        excluded: oracle::references(store, second, second_refs)?,
        packets,
    })
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

fn proof(refs: &mut BTreeSet<String>, body: &str, opening: &str, closing: &str) -> TestResult {
    let start = body.find(opening).ok_or("authored proof opening")?;
    let end = start
        .checked_add(
            body.get(start..)
                .ok_or("authored proof span")?
                .find(closing)
                .ok_or("authored proof closing")?,
        )
        .and_then(|end| end.checked_add(closing.len()))
        .ok_or("authored proof span overflow")?;
    let capture = format!("{:x}", Sha256::digest(body.as_bytes()));
    let locator = format!("html_bytes:{start}:{end}");
    let interpretation = format!(
        "{:x}",
        Sha256::digest(format!("mhsaa_subject_dcc_event_day_v1\n{capture}\n{locator}").as_bytes())
    );
    refs.extend([capture, locator, interpretation]);
    Ok(())
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
