#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

mod common;

#[path = "common/capture_cache.rs"]
mod capture_cache;

#[path = "parity_mideast_mod/compiled.rs"]
mod compiled;
#[path = "parity_mideast_mod/ks.rs"]
mod ks;
#[path = "parity_mideast_mod/ohsaa.rs"]
mod ohsaa;
#[path = "parity_mideast_mod/wayzata.rs"]
mod wayzata;

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::SchoolYear;
use census_store::Store;

const OBSERVED_ON: &str = "2026-09-20";
const SEASON: i16 = 2026;
const SEEDED_AT: &str = "2026-09-20T14:39:00Z";

fn seed_cache(cache_dir: &Path, url: &str, body: &str) -> Result<()> {
    capture_cache::seed_response(cache_dir, url, body.as_bytes(), SEEDED_AT, capture_cache::Metadata::default())
}

fn seeded(dir: &Path, bodies: &[(&str, &str)]) -> Result<(Store, Fetcher)> {
    let cache = dir.join("http");
    std::fs::create_dir_all(&cache).context("creating the seeded cache directory")?;
    for (url, body) in bodies {
        seed_cache(&cache, url, body)?;
    }
    let store = Store::open(dir.join("store")).context("opening the run's store")?;
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .context("building the fetcher")?.with_offline(true);
    Ok((store, fetcher))
}

fn context<'a>(fetcher: &'a Fetcher, store: &'a Store) -> Result<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: census_crawl::CaptureContext::LegacyMetadata,
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: OBSERVED_ON.to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
        recording: None,
    })
}
