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

const PERFORMANCES: usize = 53;

#[derive(Clone, Copy)]
enum CrossCountryFixture {
    SyntheticEmpty,
    AuthenticMissing,
}

fn seed_cache(cache_dir: &Path, url: &str, body: &str) -> Result<()> {
    use census_crawl::net::RepresentationHeaders;
    use sha2::{Digest, Sha256};

    let representation = RepresentationHeaders::canonical(&[(
        "Accept".to_string(),
        "application/json".to_string(),
    )])?;
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    hasher.update(representation.identity().as_bytes());
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "representation": representation,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
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

fn bio_url(athlete_id: u64, sport: &str) -> String {
    format!(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={athlete_id}&sport={sport}&level=4"
    )
}

struct Harness {
    _dir: tempfile::TempDir,
    fetcher: Fetcher,
    store: Store,
    registry: std::path::PathBuf,
}

impl Harness {
    fn new(cross_country: CrossCountryFixture) -> Result<Self> {
        let dir = tempfile::tempdir().context("creating a temp dir")?;
        let cache = dir.path().join("http");
        let body = fixture(SOURCE, FIXTURE)?;
        seed_cache(&cache, &bio_url(ATHLETE_ID, "tf"), &body)?;
        let xc = match cross_country {
            CrossCountryFixture::AuthenticMissing => body.clone(),
            CrossCountryFixture::SyntheticEmpty => {
                let mut value: serde_json::Value = serde_json::from_str(&body)?;
                value["dataFixture"] = json!("synthetic-explicit-empty-xc");
                value["resultsTF"] = serde_json::Value::Null;
                value["resultsXC"] = json!([]);
                serde_json::to_string(&value)?
            }
        };
        seed_cache(&cache, &bio_url(ATHLETE_ID, "xc"), &xc)?;
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
            school_year: SchoolYear::DEFAULT,
            observed_on: OBSERVED_ON.to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
            recording: None,
        };
        athleticnet::collect(&ctx, options)
            .await
            .map_err(|error| anyhow::anyhow!("the bio pull failed: {error}"))
    }
}

fn rows(store: &Store) -> Result<(Vec<CanonicalAthlete>, Vec<CanonicalPerformance>)> {
    Ok((
        store.scan(Table::Athletes)?,
        store.scan(Table::Performances)?,
    ))
}

#[test]
fn athleticnet_bio_route_absorbs_both_scopes_and_journals_each_url() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new(CrossCountryFixture::SyntheticEmpty)?;
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

            let physical_before = harness.store.stats()?.tables;
            let journal_before = harness
                .store
                .journal_payloads("athleticnet_profile_attempts_v3")?;
            let second = harness.run(&harness.options()).await?;
            ensure!(second.disposition == census_crawl::CollectionDisposition::Complete);
            ensure!(second.errors == 0 && second.unfinished.is_empty());
            let (athletes_after, performances_after) = rows(&harness.store)?;
            ensure!(
                athletes_after == athletes && performances_after == performances,
                "logical replay changes neither accepted rows nor their physical evidence"
            );
            ensure!(
                harness.store.stats()?.tables == physical_before,
                "immutable capture replay appends no physical table rows: before={physical_before:?}; after={:?}", harness.store.stats()?.tables
            );
            ensure!(
                harness
                    .store
                    .journal_payloads("athleticnet_profile_attempts_v3")?
                    == journal_before,
                "same-horizon replay preserves scope attempts"
            );
            Ok(())
        })
}

#[test]
fn authentic_missing_xc_results_remain_unfinished_after_replay() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new(CrossCountryFixture::AuthenticMissing)?;
            let first = harness.run(&harness.options()).await?;
            ensure!(first.disposition == census_crawl::CollectionDisposition::Partial);
            ensure!(first.errors == 1);
            ensure!(first.unfinished == vec![bio_url(ATHLETE_ID, "xc")]);
            let before = rows(&harness.store)?;
            ensure!(before.1.len() == PERFORMANCES);
            let physical_before = harness.store.stats()?.tables;
            let journal_before = harness
                .store
                .journal_payloads("athleticnet_profile_attempts_v3")?;
            let second = harness.run(&harness.options()).await?;
            ensure!(
                second.disposition == first.disposition && second.unfinished == first.unfinished
            );
            ensure!(rows(&harness.store)? == before);
            ensure!(harness.store.stats()?.tables == physical_before,
                "immutable partial replay changed physical rows: before={physical_before:?}; after={:?}", harness.store.stats()?.tables);
            ensure!(
                harness
                    .store
                    .journal_payloads("athleticnet_profile_attempts_v3")?
                    == journal_before
            );
            Ok(())
        })
}
