use super::{Store, Table, TestResult, UsJurisdiction};
use census_crawl::milesplit::{roster_entities, Site, TeamRef};
use census_crawl::net::FetchOutcome;
use census_domain::model::{AthleteId, SchoolId, SchoolYear};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) mod capture;
mod composed;

pub(super) fn append_for(store: &Store, athlete: &AthleteId) -> TestResult<(AthleteId, SchoolId)> {
    composed::append(store, athlete)
}

pub(super) fn append(store: &Store) -> TestResult<(AthleteId, SchoolId)> {
    let team = TeamRef {
        id: "52649".into(),
        slug: "synthetic-selected".into(),
        url: "https://wi.milesplit.com/teams/52649-synthetic-selected".into(),
        name: "Synthetic Selected High".into(),
        city_state: "Madison, WI".into(),
    };
    let body = format!(r#"<link rel="canonical" href="{}/roster"><section><h4>Current Roster</h4><ul id="rosterDataset"><li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/1-runner">Runner, Alice</a><div class="column-gender">f</div><div class="column-grad-year">2027</div><div class="data-point" data-season-id="1"><svg class="icon icon-no"></svg></div><div class="data-point" data-season-id="2"><svg class="icon icon-yes"></svg></div><div class="data-point" data-season-id="3"><svg class="icon icon-no"></svg></div></li></ul></section>"#, team.url).into_bytes();
    let outcome = capture::parse(&team, body, "2026-10-01T12:00:00Z")?;
    let capture = &outcome.capture;
    let roster = outcome.verdict.roster().ok_or("captured roster")?;
    let member = roster.athletes.first().ok_or("captured member")?;
    let published = member.athlete();
    check!(!published.indoor);
    check!(published.outdoor);
    check!(!published.xc);
    let (school, mut athletes, _) = roster_entities(
        &team,
        &roster.athletes,
        SchoolYear::DEFAULT,
        &capture.fetched_at,
        &Site::for_jurisdiction(UsJurisdiction::Wisconsin),
    )?;
    let athlete = athletes.first_mut().ok_or("roster athlete")?;
    let note = capture::row_metadata(capture, &team, "1", member.locator())?;
    let evidence = athlete
        .evidence
        .first_mut()
        .ok_or("roster source evidence")?;
    evidence.source.url = Some(capture.url.clone());
    evidence.note = Some(note);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, athlete)?;
    retain(store, capture)?;
    Ok((athlete.id.clone(), school.id))
}

pub(crate) fn retain(store: &Store, capture: &FetchOutcome) -> TestResult {
    let root = store.http_cache_dir().join("archive");
    let captures = root.join("captures").join(&capture.content_digest);
    std::fs::create_dir_all(root.join("bodies"))?;
    std::fs::create_dir_all(&captures)?;
    std::fs::write(
        root.join("bodies")
            .join(format!("{}.body", capture.content_digest)),
        &capture.body,
    )?;
    let metadata = serde_json::to_vec(&json!({
        "url": capture.url, "response_url": null, "method": "GET", "status": 200,
        "content_digest": capture.content_digest, "bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "content_type": "text/html", "etag": "synthetic fixture", "redirects": [],
    }))?;
    std::fs::write(
        captures.join(format!("{}.meta.json", hash(&metadata))),
        metadata,
    )?;
    Ok(())
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
