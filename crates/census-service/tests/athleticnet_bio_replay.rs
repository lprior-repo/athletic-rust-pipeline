//! The per-athlete bio route, end to end: `athleticnet::collect` driven through `Fetcher` against
//! the anonymized capture of athlete 28872883's biography, with no socket in the run.
//!
//! Both (athlete, sport) request URLs are seeded into the fetcher's on-disk cache, so every request
//! is served from disk and the run proves the whole route — the registry read, URL construction, the
//! payload decode, the walk, the batch append and the journal — rather than the mapper alone. The
//! fixture is the anonymized capture of `research/sources/athleticnet/samples/
//! live-getathletebiodata-28872883-2026-09-20.json`; its cross-country half is `null` (the athlete
//! has no XC rows), so the XC scope absorbs nothing and the TF half carries every storable row.
//!
//! The second run proves replay: the journal short-circuits both URLs, so no request is built, no
//! cached body is read, and no row is appended twice.
//!
//! `AdapterReport::requests` is the fetcher's own count of **network** requests, so a fixture-driven
//! run reports `0` there and names the served documents in `from_cache` — which is the point: no
//! socket is opened by this test, and the numbers it asserts are the ones the run really spent.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::athleticnet::{self, Options};
use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::json;

/// Reads `<crawl crate>/tests/fixtures/<source>/<file>` without linking the golden-corpus harness.
fn fixture(source: &str, file: &str) -> Result<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures")
        .join(source)
        .join(file);
    fs::read_to_string(&path).with_context(|| format!("reading fixture {}", path.display()))
}

const OBSERVED_ON: &str = "2026-09-22";
const ATHLETE_ID: u64 = 28872883;
const SOURCE: &str = "athleticnet";
const FIXTURE: &str = "bio_28872883_tf.json";

/// The capture's storable performance rows: the pinned number this route stores, so a silent shape
/// change in `absorb` cannot pass as a green run.
const PERFORMANCES: usize = 53;

/// Seed the fetcher's on-disk cache for `url` under the key `Fetcher` derives
/// (`sha256(method \x1f url \x1f body)[..16]`), so `collect` can be driven without a socket.
fn seed_cache(cache_dir: &Path, url: &str, body: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-22T12:00:00Z",
    });
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating cache dir {}", cache_dir.display()))?;
    fs::write(cache_dir.join(format!("{key}.body")), body)
        .with_context(|| format!("writing the cached body for {url}"))?;
    fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta).context("serializing cache metadata")?,
    )
    .with_context(|| format!("writing the cached metadata for {url}"))?;
    Ok(())
}

/// The bio URL the route builds: `{BIO_ENDPOINT}?athleteId={id}&sport={sport}&level=4`.
///
/// The endpoint and level constants are private to the adapter, so the test spells the URL the
/// adapter's own `every_acquisition_endpoint_is_browser_transported` test pins.
fn bio_url(athlete_id: u64, sport: &str) -> String {
    format!(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={athlete_id}&sport={sport}&level=4"
    )
}

/// A run's harness: the cache every request is served from, the store it writes, and the fetcher.
struct Harness {
    _dir: tempfile::TempDir,
    fetcher: Fetcher,
    store: Store,
    registry: std::path::PathBuf,
}

impl Harness {
    /// The harness for one athlete, with both (athlete, sport) documents seeded.
    fn new() -> Result<Self> {
        let dir = tempfile::tempdir().context("creating a temp dir")?;
        let cache = dir.path().join("http");
        let body = fixture(SOURCE, FIXTURE)?;
        for sport in ["tf", "xc"] {
            seed_cache(&cache, &bio_url(ATHLETE_ID, sport), &body)?;
        }
        let store = Store::open(dir.path().join("store"))?;
        let fetcher = Fetcher::new(
            &cache,
            None,
            Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )?;
        let registry = dir.path().join("registry.txt");
        fs::write(&registry, format!("{ATHLETE_ID},AK\n")).context("writing the registry")?;
        Ok(Self {
            _dir: dir,
            fetcher,
            store,
            registry,
        })
    }

    fn options(&self) -> Options {
        Options {
            input: Some(self.registry.display().to_string()),
            states: vec![UsJurisdiction::Alaska],
            observed_on: OBSERVED_ON.to_string(),
            ..Options::default()
        }
    }

    async fn run(&self, options: &Options) -> Result<census_crawl::AdapterReport> {
        let ctx = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).expect("2026 is a season"),
            observed_on: OBSERVED_ON.to_string(),
            recording: None,
        };
        athleticnet::collect(&ctx, options)
            .await
            .map_err(|error| anyhow::anyhow!("the bio pull failed: {error}"))
    }
}

/// The store's row counts for the tables a bio run writes, plus the athlete row itself.
fn rows(store: &Store) -> Result<(Vec<CanonicalAthlete>, Vec<CanonicalPerformance>)> {
    Ok((
        store.scan(Table::Athletes)?,
        store.scan(Table::Performances)?,
    ))
}

#[tokio::test]
async fn athleticnet_bio_route_absorbs_both_scopes_and_journals_each_url() -> Result<()> {
    let harness = Harness::new()?;
    let first = harness.run(&harness.options()).await?;
    ensure!(
        first.requests == 0 && first.from_cache == 2,
        "both documents are served from the cache: {} requests, {} served",
        first.requests,
        first.from_cache
    );
    ensure!(
        first.errors == 0,
        "the payload decodes without a failure: {:?}",
        first.notes
    );
    ensure!(
        first.rows == 1 && first.unit == "athletes",
        "one athlete is absorbed: {} {}",
        first.rows,
        first.unit
    );
    let (athletes, performances) = rows(&harness.store)?;
    ensure!(
        athletes.len() == 1,
        "one athlete from the capture's registry line: {athletes:?}"
    );
    ensure!(
        performances.len() == PERFORMANCES,
        "the capture stores {PERFORMANCES} storable performances: {}",
        performances.len()
    );

    let second = harness.run(&harness.options()).await?;
    ensure!(
        second.requests == 0 && second.from_cache == 0,
        "the journal short-circuits both URLs: {} requests, {} served",
        second.requests,
        second.from_cache
    );
    ensure!(
        second.rows == 0,
        "and nothing is walked twice: {} rows",
        second.rows
    );
    let (athletes_after, performances_after) = rows(&harness.store)?;
    ensure!(
        (athletes_after.len(), performances_after.len()) == (athletes.len(), performances.len()),
        "the store still holds one copy of every row: {} athletes, {} performances",
        athletes_after.len(),
        performances_after.len()
    );
    Ok(())
}
