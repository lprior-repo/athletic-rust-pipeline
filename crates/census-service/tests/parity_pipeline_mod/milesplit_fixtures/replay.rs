use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use census_crawl::{milesplit, net::Fetcher, AdapterContext};
use census_domain::model::{CanonicalSchool, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::Store;

use super::{RAW_FILES, RELAYS, TROY_URL};
use crate::common;

#[path = "replay/assertions.rs"]
mod assertions;
#[path = "replay/cache.rs"]
mod cache;

const OWNED: &[u8] =
    include_bytes!("../../../../census-crawl/src/milesplit/owned/fixtures/troy_725218.json");
const ACQUIRED_AT: &str = "2026-10-01T23:44:16Z";
const PHASE: &str = milesplit::RESULT_SET_PHASE;

pub async fn replay_owned_captures() -> Result<()> {
    let school = fixture_school()?;
    let relays = common::fixture("milesplit", RELAYS)?;
    replay_capture(&school, OWNED, false).await?;
    replay_capture(&school, relays.as_bytes(), true).await
}

fn fixture_school() -> Result<CanonicalSchool> {
    let teams = milesplit::parse_team_index(&common::fixture("milesplit", "wi_teams_index.html")?)?;
    let team = teams
        .into_iter()
        .find(|team| team.id == "52649")
        .context("Wisconsin source team missing")?;
    let verdict =
        milesplit::parse_roster(&common::fixture("milesplit", "wi_roster_52649.html")?, team)?;
    let roster = verdict.roster().context("Wisconsin roster quarantined")?;
    let site = milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let school_year = SchoolYear::new(2026).context("2026 school year")?;
    Ok(milesplit::roster_entities(roster, school_year, "2026-09-20", &site).0)
}

async fn replay_capture(school: &CanonicalSchool, body: &[u8], relay: bool) -> Result<()> {
    let root = tempfile::tempdir().context("isolated retained-capture replay")?;
    let store = Store::open(root.path())?;
    std::fs::create_dir_all(store.out_dir())?;
    std::fs::write(
        store.out_dir().join("schools.jsonl"),
        format!("{}\n", serde_json::to_string(school)?),
    )?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let options = seed_captures(&fetcher, body)?;
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).context("2026 school year")?,
        observed_on: "2026-09-20".to_string(),
        recording: None,
    };
    let report = milesplit::collect_result_sets(&context, &options).await?;
    check!(eq; report.requests, 0);
    check!(eq; report.from_cache, 3);
    check!(eq; report.rows, if relay { 0 } else { 3 });
    check!(eq; report.errors, if relay { 0 } else { 1 });
    assertions::assert_retention(&store, body, relay)?;
    let before = store.journal_payloads(PHASE)?;
    let repeated = milesplit::collect_result_sets(&context, &options).await?;
    check!(eq; repeated.requests, 0);
    check!(eq; store.journal_payloads(PHASE)?, before);
    assertions::assert_retention(&store, body, relay)
}

fn seed_captures(fetcher: &Fetcher, body: &[u8]) -> Result<milesplit::ResultSetOptions> {
    let owned_url = format!(
        "https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        milesplit::OWNED_FIELDS
    );
    cache::seed(fetcher.cache_dir(), &owned_url, body, ACQUIRED_AT)?;
    let mut urls = Vec::new();
    for (file, rsid, acquired_at) in [
        (RAW_FILES[0], "1266814", "2026-09-28T10:11:24Z"),
        (RAW_FILES[1], "1266815", "2026-09-28T10:11:13Z"),
    ] {
        let url = format!("{TROY_URL}/{rsid}/raw");
        cache::seed(
            fetcher.cache_dir(),
            &url,
            common::fixture("milesplit", file)?.as_bytes(),
            acquired_at,
        )?;
        urls.push(milesplit::ResultSetRequest {
            url,
            jurisdiction: UsJurisdiction::Alabama,
        });
    }
    Ok(milesplit::ResultSetOptions { urls })
}
