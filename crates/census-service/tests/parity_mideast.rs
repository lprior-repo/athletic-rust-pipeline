#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

mod common;

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
    use sha2::{Digest as _, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": SEEDED_AT,
    });

    let dir = cache_dir.to_path_buf();
    std::fs::write(
        dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta).context("serializing the seeded cache metadata")?,
    )
    .with_context(|| format!("writing the cache metadata for {url}"))?;
    std::fs::write(dir.join(format!("{key}.body")), body)
        .with_context(|| format!("writing the cache body for {url}"))?;
    Ok(())
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
    .context("building the fetcher")?;
    Ok((store, fetcher))
}

fn context<'a>(fetcher: &'a Fetcher, store: &'a Store) -> Result<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: OBSERVED_ON.to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
        recording: None,
    })
}
