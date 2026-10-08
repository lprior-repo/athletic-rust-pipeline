use std::collections::{BTreeMap, BTreeSet, HashMap};
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

fn fixture(source: &str, file: &str) -> Result<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures")
        .join(source)
        .join(file);
    fs::read_to_string(&path).with_context(|| format!("reading fixture {}", path.display()))
}

const OBSERVED_ON: &str = "2026-09-22";
const CAPTURED_AT: &str = "2026-09-22T12:00:00Z";
const MEET_ID: i64 = 634313;
const SOURCE: &str = "athleticnet";

fn seed_cache(cache_dir: &Path, url: &str, token: Option<&str>, body: &str) -> Result<()> {
    use census_crawl::net::RepresentationHeaders;
    use sha2::{Digest, Sha256};

    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
    if let Some(token) = token {
        headers.push(("anettokens".to_string(), token.to_string()));
    }
    let representation = RepresentationHeaders::canonical(&headers)?;
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
        "fetched_at": CAPTURED_AT,
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

struct Harness {
    _dir: tempfile::TempDir,
    fetcher: Fetcher,
    store: Store,
}

impl Harness {
    fn new(event_metadata: bool) -> Result<Self> {
        let dir = tempfile::tempdir().context("creating a temp dir")?;
        let cache = dir.path().join("http");
        let [meet_url, results_url] = meet_requests(MEET_ID);
        let meet_body = fixture(SOURCE, "meet_634313_meetdata.json")?;
        let token = serde_json::from_str::<athleticnet::MeetData>(&meet_body)
            .context("parsing the meet fixture's token")?
            .token;
        seed_cache(&cache, &meet_url, None, &meet_body)?;
        seed_cache(
            &cache,
            &results_url,
            token.as_deref(),
            &fixture(SOURCE, "meet_634313_allresults.json")?,
        )?;
        if event_metadata {
            seed_cache(
                &cache,
                &metadata_request(MEET_ID),
                token.as_deref(),
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
            school_year: SchoolYear::DEFAULT,
            observed_on: OBSERVED_ON.to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
            recording: None,
        };
        athleticnet::collect(&ctx, options)
            .await
            .map_err(|error| anyhow::anyhow!("the meet pull failed: {error}"))
    }
}

fn captured_subject_keys() -> Result<BTreeSet<String>> {
    let capture: athleticnet::AllResults =
        serde_json::from_str(&fixture(SOURCE, "meet_634313_allresults.json")?)?;
    let relay_parents: BTreeSet<_> = capture.legs.iter().map(|leg| leg.result_id).collect();
    let mut subjects = BTreeSet::new();
    for row in capture.blocks.iter().flat_map(|block| &block.results) {
        if !relay_parents.contains(&row.result_id) {
            let owner = row
                .athlete_id
                .context("captured individual has no native owner")?;
            ensure!(
                subjects.insert(format!("athleticnet:{owner}-{}", row.result_id)),
                "duplicate published individual"
            );
        }
    }
    let mut positions = BTreeMap::<i64, usize>::new();
    for leg in &capture.legs {
        let position = positions.entry(leg.result_id).or_insert(0);
        *position = position.checked_add(1).context("relay position overflow")?;
        let owner = leg
            .athlete_id
            .context("captured relay member has no native owner")?;
        ensure!(
            subjects.insert(format!(
                "athleticnet:{owner}-{}:leg{position}",
                leg.result_id
            )),
            "duplicate published relay member"
        );
    }
    Ok(subjects)
}

fn exact_capture_subjects(rows: &[CanonicalPerformance]) -> Result<()> {
    let actual: BTreeSet<_> = rows.iter().map(|row| row.source_key.clone()).collect();
    ensure!(
        actual.len() == rows.len(),
        "physical duplicate native result subjects"
    );
    ensure!(
        actual == captured_subject_keys()?,
        "captured native participant lost or invented"
    );
    Ok(())
}

#[test]
fn athleticnet_whole_meet_route_pulls_two_requests_and_stores_every_storable_row() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new(false)?;
            let report = harness.run(&harness.options(false)).await?;
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
                report.rows == u64::try_from(captured_subject_keys()?.len())?,
                "every independently inventoried native result subject is retained: {}",
                report.rows
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
            let subjects = captured_subject_keys()?;
            let expected_owners: BTreeSet<_> = subjects
                .iter()
                .filter_map(|key| {
                    key.strip_prefix("athleticnet:")
                        .and_then(|key| key.split_once('-'))
                })
                .map(|(owner, _)| owner)
                .collect();
            let actual_owners: BTreeSet<_> = athletes
                .iter()
                .flat_map(CanonicalAthlete::identities)
                .filter(|source| {
                    source.namespace
                        == census_domain::model::SourceNamespace::athletic_net("athlete")
                })
                .map(|source| source.id.as_str())
                .collect();
            ensure!(
                actual_owners == expected_owners,
                "captured native participant owner lost or invented"
            );
            let performances: Vec<CanonicalPerformance> =
                harness.store.scan(Table::Performances)?;
            exact_capture_subjects(&performances)?;
            let legs: Vec<&CanonicalPerformance> = performances
                .iter()
                .filter(|row| row.source_key.contains(":leg"))
                .collect();
            ensure!(
                legs.iter()
                    .map(|row| &row.source_key)
                    .collect::<BTreeSet<_>>()
                    == captured_subject_keys()?
                        .iter()
                        .filter(|key| key.contains(":leg"))
                        .collect::<BTreeSet<_>>(),
                "every published native relay member has exactly one result subject"
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
                    .is_some_and(|evidence| evidence.observed_on == CAPTURED_AT
                        && evidence.source.id == SOURCE)),
                "every row carries its original physical acquisition evidence"
            );
            Ok(())
        })
}

#[test]
fn athleticnet_third_request_is_spent_only_when_asked_and_changes_no_row() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
                report.rows == u64::try_from(captured_subject_keys()?.len())?,
                "optional event metadata loses or invents no captured result subject: {}",
                report.rows
            );
            let performances: Vec<CanonicalPerformance> =
                harness.store.scan(Table::Performances)?;
            exact_capture_subjects(&performances)?;
            Ok(())
        })
}

#[test]
fn athleticnet_second_run_resumes_from_the_journal_and_spends_nothing() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new(false)?;
            let first = harness.run(&harness.options(false)).await?;
            ensure!(
                first.rows == u64::try_from(captured_subject_keys()?.len())?,
                "first capture subject conservation"
            );
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
            let performances: Vec<CanonicalPerformance> =
                harness.store.scan(Table::Performances)?;
            exact_capture_subjects(&performances)?;
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
        })
}
