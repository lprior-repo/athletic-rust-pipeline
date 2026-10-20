use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use census_crawl::{milesplit, net::Fetcher, AdapterContext, CaptureContext};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Table;

use super::{RAW_FILES, RELAYS, TROY_URL};
use crate::common;

#[path = "replay/assertions.rs"]
mod assertions;
#[path = "../../common/capture_cache.rs"]
mod cache;

pub(crate) use census_crawl::milesplit::troy_fixture::BODY as OWNED;
pub(crate) const ACQUIRED_AT: &str = "2026-10-01T23:44:16Z";
pub(crate) const RAW_CAPTURES: [(&str, &str, &str); 2] = [
    (RAW_FILES[0], "1266814", "2026-09-28T10:11:24Z"),
    (RAW_FILES[1], "1266815", "2026-09-28T10:11:13Z"),
];
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
    let durable = crate::durable_capture::DurableCapture::mint()?;
    let store = durable.store().clone();
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let options = seed_captures(&durable, &fetcher, body)?;
    let owned = durable.capture()?;
    let context = AdapterContext {
        capture: CaptureContext::DurableV1(&owned),
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
    check!(eq; report.from_cache, 2);
    check!(eq; report.rows, if relay { 0 } else { 3 });
    check!(eq; report.errors, if relay { 0 } else { 1 });
    assertions::assert_retention(&store, body, relay)?;
    let before = store.journal_payloads(PHASE)?;
    let observations = store.snapshot().tables_digest(&Table::ALL)?;
    let source_meets = store.journal_payloads(milesplit::OWNED_MEET_PHASE)?;
    let captures = store.journal_payloads(milesplit::OWNED_CAPTURE_PHASE)?;
    let replayed = durable.capture()?;
    let context = AdapterContext {
        capture: CaptureContext::DurableV1(&replayed),
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).context("2026 school year")?,
        observed_on: "2026-09-20".to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-09-20", "%Y-%m-%d")?,
        recording: None,
    };
    let repeated = milesplit::collect_result_sets(&context, &options).await?;
    check!(eq; repeated.requests, 0);
    check!(eq; store.journal_payloads(PHASE)?, before);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, observations);
    check!(eq; store.journal_payloads(milesplit::OWNED_MEET_PHASE)?, source_meets);
    check!(eq; store.journal_payloads(milesplit::OWNED_CAPTURE_PHASE)?, captures);
    assertions::assert_retention(&store, body, relay)
}

pub(crate) fn seed_captures(
    durable: &crate::durable_capture::DurableCapture,
    fetcher: &Fetcher,
    body: &[u8],
) -> Result<milesplit::ResultSetOptions> {
    let mut urls = Vec::new();
    let mut owned_reference = None;
    for (file, rsid, acquired_at) in RAW_CAPTURES {
        let url = format!("{TROY_URL}/{rsid}/raw");
        if owned_reference.is_none() {
            owned_reference = milesplit::ResultSetRef::parse(&url);
        }
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
    let reference = owned_reference.context("owned Troy reference")?;
    durable.record_owned_meet(fetcher, &reference, body, ACQUIRED_AT)?;
    Ok(milesplit::ResultSetOptions { urls })
}
