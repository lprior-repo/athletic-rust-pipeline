use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use census_crawl::{milesplit, net::Fetcher, AdapterContext};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;

use super::{RAW_FILES, RELAYS, TROY_URL};
use crate::common;

#[path = "replay/assertions.rs"]
mod assertions;
#[path = "../../common/capture_cache.rs"]
mod cache;

const OWNED: &[u8] =
    include_bytes!("../../../../census-crawl/src/milesplit/owned/fixtures/troy_725218.json");
const ACQUIRED_AT: &str = "2026-10-01T23:44:16Z";
const PHASE: &str = milesplit::RESULT_SET_PHASE;

pub async fn replay_owned_captures() -> Result<()> {
    let relays = common::fixture("milesplit", RELAYS)?;
    let provenance = common::fixture("milesplit", super::PROVENANCE)?;
    check!(super::validate_result_fixture(
        super::PROVENANCE,
        &provenance
    )?);
    check!(super::validate_result_fixture(RELAYS, &relays)?);
    replay_capture(OWNED, false).await?;
    replay_capture(relays.as_bytes(), true).await
}

async fn replay_capture(body: &[u8], relay: bool) -> Result<()> {
    let root = tempfile::tempdir().context("isolated retained-capture replay")?;
    let store = Store::open(root.path())?;
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
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-09-20", "%Y-%m-%d")?,
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
    cache::seed(fetcher.cache_dir(), &owned_url, body, ACQUIRED_AT, &[])?;
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
            &[],
        )?;
        urls.push(milesplit::ResultSetRequest {
            url,
            jurisdiction: UsJurisdiction::Alabama,
        });
    }
    Ok(milesplit::ResultSetOptions { urls })
}
