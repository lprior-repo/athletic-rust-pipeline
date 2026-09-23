//! The whole-meet pull, end to end: `athleticnet::collect` driven through `Fetcher` against the
//! genuine probe capture of meet 634313, with no socket in the run.
//!
//! The two (and, with `event_metadata`, three) request URLs are seeded into the fetcher's on-disk
//! cache, so every request is served from disk and the run proves the whole route — URL
//! construction, the `anettokens` header path, the journal, the walk, and the store append —
//! rather than the mapper alone. The fixture bodies are the anonymous captures of
//! `research/sources/athleticnet/samples/anon-{meetdata,allresults,eventdiv}-634313.json`, whose
//! measured shape is pinned in `crates/census-crawl/src/athleticnet/meet/tests.rs` and in that module's doc:
//! 758 published rows, 72 relay squads, 288 legs, 903 storable performances.
//!
//! Both routes are exercised: the registry route is *not* reached, because `Options.meets` is set,
//! and a second run over the same store proves the journal short-circuits the request pair.
//!
//! `AdapterReport::requests` is the fetcher's own count of **network** requests, so a fixture-driven
//! run reports `0` there and names the served documents in `from_cache` — which is the point: no
//! socket is opened by this test, and the numbers it asserts are the ones the run really spent.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::athleticnet::{self, meet_requests, metadata_request, Options};
use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, SchoolYear, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::json;

/// Reads `<crawl crate>/tests/fixtures/<source>/<file>` without linking the golden-corpus harness:
/// this test needs one fixture per request and none of `common`'s golden machinery.
fn fixture(source: &str, file: &str) -> Result<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures")
        .join(source)
        .join(file);
    fs::read_to_string(&path).with_context(|| format!("reading fixture {}", path.display()))
}

const OBSERVED_ON: &str = "2026-09-22";
const MEET_ID: i64 = 634313;
const SOURCE: &str = "athleticnet";

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

/// A run's harness: the cache every request is served from, the store it writes, and the fetcher.
struct Harness {
    _dir: tempfile::TempDir,
    fetcher: Fetcher,
    store: Store,
}

impl Harness {
    /// The harness for one meet, with the metadata document seeded only when `event_metadata`.
    fn new(event_metadata: bool) -> Result<Self> {
        let dir = tempfile::tempdir().context("creating a temp dir")?;
        let cache = dir.path().join("http");
        let [meet_url, results_url] = meet_requests(MEET_ID);
        seed_cache(
            &cache,
            &meet_url,
            &fixture(SOURCE, "meet_634313_meetdata.json")?,
        )?;
        seed_cache(
            &cache,
            &results_url,
            &fixture(SOURCE, "meet_634313_allresults.json")?,
        )?;
        if event_metadata {
            seed_cache(
                &cache,
                &metadata_request(MEET_ID),
                &fixture(SOURCE, "meet_634313_eventdiv.json")?,
            )?;
        }
        let store = Store::open(dir.path().join("store"))?;
        let fetcher = Fetcher::new(
            &cache,
            None,
            Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )?;
        Ok(Self {
            _dir: dir,
            fetcher,
            store,
        })
    }

    fn options(&self, event_metadata: bool) -> Options {
        Options {
            meets: vec![MEET_ID],
            event_metadata,
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
            .map_err(|error| anyhow::anyhow!("the meet pull failed: {error}"))
    }
}

#[tokio::test]
async fn athleticnet_whole_meet_route_pulls_two_requests_and_stores_every_storable_row(
) -> Result<()> {
    let harness = Harness::new(false)?;
    let report = harness.run(&harness.options(false)).await?;
    ensure!(
        report.adapter == SOURCE,
        "the report names the adapter: {}",
        report.adapter
    );
    ensure!(
        report.unit == "performances",
        "the whole-meet route counts performances, not athletes: {}",
        report.unit
    );
    ensure!(
        report.requests == 0,
        "no socket was opened: {} requests went to the network",
        report.requests
    );
    ensure!(
        report.from_cache == 2,
        "one meet is two requests, both served from the seeded cache: {}",
        report.from_cache
    );
    ensure!(report.errors == 0, "no request failed: {}", report.errors);
    ensure!(
        report.rows == 903,
        "623 individual results plus 280 relay legs: {}",
        report.rows
    );
    ensure!(
        report
            .notes
            .iter()
            .any(|note| note.contains("758 seen") && note.contains("288 legs seen")),
        "the run report carries the walk's own counters: {:?}",
        report.notes
    );
    ensure!(
        report
            .notes
            .iter()
            .any(|note| note.contains("the third request is off by default")),
        "the run says the third request was not spent: {:?}",
        report.notes
    );

    let meets: Vec<CanonicalMeet> = harness.store.scan(Table::Meets)?;
    ensure!(meets.len() == 1, "one meet row: {}", meets.len());
    ensure!(
        meets[0].state == Some(UsJurisdiction::Wisconsin),
        "the published venue is WI: {:?}",
        meets[0].state
    );
    ensure!(
        meets[0].name == "Big 8 Conference",
        "the published name: {}",
        meets[0].name
    );
    let athletes: Vec<CanonicalAthlete> = harness.store.scan(Table::Athletes)?;
    ensure!(
        athletes.len() == 490,
        "490 athletes keyed by (athlete id, school): {}",
        athletes.len()
    );
    let performances: Vec<CanonicalPerformance> = harness.store.scan(Table::Performances)?;
    ensure!(
        performances.len() == 903,
        "903 performances: {}",
        performances.len()
    );
    let legs: Vec<&CanonicalPerformance> = performances
        .iter()
        .filter(|row| row.source_key.contains(":leg"))
        .collect();
    ensure!(
        legs.len() == 280,
        "280 relay legs, each its own row: {}",
        legs.len()
    );
    ensure!(
        legs.iter().all(|row| row
            .evidence
            .first()
            .and_then(|evidence| evidence.note.as_deref())
            .is_some_and(|note| note.starts_with("relay leg "))),
        "every leg says so in its own evidence"
    );
    ensure!(
        performances.iter().all(|row| row
            .observed_grade
            .is_some_and(|grade| (9..=12).contains(&grade.get()))),
        "no stored performance carries a placeholder grade"
    );
    ensure!(
        athletes
            .iter()
            .all(|athlete| !athlete.canonical_name.contains("<BR>")),
        "no squad's padded name became a person"
    );
    ensure!(
        performances.iter().all(|row| row
            .evidence
            .first()
            .is_some_and(
                |evidence| evidence.observed_on == OBSERVED_ON && evidence.source.id == SOURCE
            )),
        "every row carries this run's evidence"
    );
    Ok(())
}

#[tokio::test]
async fn athleticnet_third_request_is_spent_only_when_asked_and_changes_no_row() -> Result<()> {
    let harness = Harness::new(true)?;
    let report = harness.run(&harness.options(true)).await?;
    ensure!(
        report.from_cache == 3,
        "the metadata document is a third request: {} served",
        report.from_cache
    );
    ensure!(
        report.requests == 0,
        "still no socket: {} went to the network",
        report.requests
    );
    ensure!(
        report.rows == 903,
        "and it changes no row: {} stored",
        report.rows
    );
    ensure!(
        report.notes.iter().any(
            |note| note.contains("36 events declared, 12 of them field events, 4 hurdle races")
        ),
        "the run reports what the metadata document declared: {:?}",
        report.notes
    );
    let performances: Vec<CanonicalPerformance> = harness.store.scan(Table::Performances)?;
    ensure!(
        performances.len() == 903,
        "903 performances with the third request spent: {}",
        performances.len()
    );
    Ok(())
}

#[tokio::test]
async fn athleticnet_second_run_resumes_from_the_journal_and_spends_nothing() -> Result<()> {
    let harness = Harness::new(false)?;
    let first = harness.run(&harness.options(false)).await?;
    ensure!(first.rows == 903, "the first run stores: {}", first.rows);
    let second = harness.run(&harness.options(false)).await?;
    ensure!(
        second.requests == 0 && second.from_cache == 0,
        "the journal short-circuits the pair: {} requests, {} served",
        second.requests,
        second.from_cache
    );
    ensure!(
        second.rows == 0,
        "and nothing is walked twice: {} rows",
        second.rows
    );
    ensure!(
        second
            .notes
            .iter()
            .any(|note| note.contains("meets: 0 pulled")),
        "the run says the meet was already journaled: {:?}",
        second.notes
    );
    let performances: Vec<CanonicalPerformance> = harness.store.scan(Table::Performances)?;
    ensure!(
        performances.len() == 903,
        "the store still holds exactly one copy of every row: {}",
        performances.len()
    );
    let sports: Vec<Sport> = harness
        .store
        .scan::<CanonicalMeet>(Table::Meets)?
        .into_iter()
        .flat_map(|meet| meet.sports)
        .collect();
    ensure!(
        sports == vec![Sport::OutdoorTrack],
        "the meet's sport comes from the payload's `sport2`: {sports:?}"
    );
    Ok(())
}
