use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use census_crawl::athleticnet::{self, meet_requests, metadata_request, Options};
use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, RelayResult, SchoolYear, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[path = "common/capture_cache.rs"]
mod capture_cache;
#[path = "common/durable_capture.rs"]
mod durable_capture;

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
    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
    if let Some(token) = token {
        headers.push(("anettokens".to_string(), token.to_string()));
    }
    capture_cache::seed(cache_dir, url, body.as_bytes(), CAPTURED_AT, &headers)
}

struct Harness {
    _dir: tempfile::TempDir,
    fetcher: Fetcher,
    store: Store,
    durable: durable_capture::DurableCapture,
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
        )?
        .with_offline(true);
        Ok(Self {
            _dir: dir,
            fetcher,
            store,
            durable: durable_capture::DurableCapture::mint()
                .map_err(|error| anyhow::anyhow!("minting the durable capture: {error}"))?,
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
        let owned = self
            .durable
            .capture()
            .map_err(|error| anyhow::anyhow!("minting the durable capture: {error}"))?;
        let ctx = AdapterContext {
            capture: census_crawl::CaptureContext::DurableV1(&owned),
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

fn captured_individual_keys() -> Result<BTreeSet<String>> {
    let capture: athleticnet::AllResults =
        serde_json::from_str(&fixture(SOURCE, "meet_634313_allresults.json")?)?;
    let relay_parents: BTreeSet<_> = capture.legs.iter().map(|leg| leg.result_id).collect();
    let mut subjects = BTreeSet::new();
    for row in capture.blocks.iter().flat_map(|block| &block.results) {
        if relay_parents.contains(&row.result_id) {
            continue;
        }
        let owner = row
            .athlete_id
            .context("captured individual has no native owner")?;
        ensure!(
            subjects.insert(format!("athleticnet:{owner}-{}", row.result_id)),
            "duplicate published individual"
        );
    }
    Ok(subjects)
}

fn captured_relay_parents() -> Result<BTreeMap<i64, i64>> {
    let capture: athleticnet::AllResults =
        serde_json::from_str(&fixture(SOURCE, "meet_634313_allresults.json")?)?;
    let relay_parents: BTreeSet<_> = capture.legs.iter().map(|leg| leg.result_id).collect();
    let mut parents = BTreeMap::new();
    for row in capture.blocks.iter().flat_map(|block| &block.results) {
        if !relay_parents.contains(&row.result_id) {
            continue;
        }
        let team = row.team_id.context("captured relay row has no team")?;
        match parents.entry(row.result_id) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(team);
            }
            std::collections::btree_map::Entry::Occupied(slot) => {
                ensure!(
                    *slot.into_mut() == team,
                    "relay squad changes teams: {}",
                    row.result_id
                );
            }
        }
    }
    ensure!(
        parents.len() == relay_parents.len(),
        "every relay parent publishes a squad row"
    );
    Ok(parents)
}

fn captured_leg_members() -> Result<Vec<(i64, usize, String)>> {
    let capture: athleticnet::AllResults =
        serde_json::from_str(&fixture(SOURCE, "meet_634313_allresults.json")?)?;
    let mut positions = BTreeMap::<i64, usize>::new();
    let mut members = Vec::new();
    for leg in &capture.legs {
        let position = positions.entry(leg.result_id).or_insert(0);
        *position = position.checked_add(1).context("relay position overflow")?;
        leg.athlete_id
            .context("captured relay member has no native owner")?;
        members.push((leg.result_id, *position, leg.name.clone()));
    }
    Ok(members)
}

fn exact_capture_subjects(rows: &[CanonicalPerformance]) -> Result<()> {
    let actual: BTreeSet<_> = rows.iter().map(|row| row.source_key.clone()).collect();
    ensure!(
        actual.len() == rows.len(),
        "physical duplicate native result subjects"
    );
    ensure!(
        actual == captured_individual_keys()?,
        "captured native participant lost or invented"
    );
    Ok(())
}

fn exact_capture_relays(relays: &[RelayResult], athletes: &[CanonicalAthlete]) -> Result<()> {
    let parents = captured_relay_parents()?;
    let expected_keys: BTreeSet<_> = parents
        .iter()
        .map(|(result_id, team_id)| format!("athleticnet:team{team_id}-{result_id}"))
        .collect();
    let actual_keys: BTreeSet<_> = relays
        .iter()
        .map(|relay| relay.source_key.clone())
        .collect();
    ensure!(
        actual_keys.len() == relays.len(),
        "physical duplicate relay squad"
    );
    ensure!(actual_keys == expected_keys, "relay squad lost or invented");
    let members = captured_leg_members()?;
    ensure!(
        relays
            .iter()
            .map(|relay| relay.members.len())
            .sum::<usize>()
            == members.len(),
        "every published relay leg is retained as a squad member"
    );
    let mut expected: BTreeMap<(i64, usize), String> = BTreeMap::new();
    for (result_id, position, name) in members {
        ensure!(
            expected.insert((result_id, position), name).is_none(),
            "duplicate published relay member"
        );
    }
    let key_of = |result_id: i64| {
        let team_id = parents.get(&result_id).copied().unwrap_or_default();
        format!("athleticnet:team{team_id}-{result_id}")
    };
    for relay in relays {
        let result_id = parents
            .iter()
            .find(|(result_id, _)| relay.source_key == key_of(**result_id))
            .map(|(result_id, _)| *result_id)
            .context("relay squad without a captured parent")?;
        for (index, member) in relay.members.iter().enumerate() {
            let position = index.checked_add(1).context("relay position overflow")?;
            let name = expected
                .remove(&(result_id, position))
                .context("squad member without a published leg")?;
            ensure!(
                member.order == u32::try_from(position)?,
                "relay member order follows the published leg order"
            );
            ensure!(
                member.name_as_published == name,
                "relay member keeps its published name"
            );
            let athlete = member
                .athlete
                .as_ref()
                .context("eligible relay member without a linked athlete")?;
            ensure!(
                athletes.iter().any(|row| &row.id == athlete),
                "linked relay member athlete is retained"
            );
        }
    }
    ensure!(
        expected.is_empty(),
        "published relay legs without a retained squad member"
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
                report.rows
                    == u64::try_from(
                        captured_individual_keys()?.len() + captured_relay_parents()?.len()
                    )?,
                "individual results plus relay squads are retained: {}",
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
            let relays: Vec<RelayResult> = harness.store.scan(Table::RelayResults)?;
            exact_capture_relays(&relays, &athletes)?;
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
                report.rows
                    == u64::try_from(
                        captured_individual_keys()?.len() + captured_relay_parents()?.len()
                    )?,
                "optional event metadata loses or invents no captured subject: {}",
                report.rows
            );
            let performances: Vec<CanonicalPerformance> =
                harness.store.scan(Table::Performances)?;
            exact_capture_subjects(&performances)?;
            let athletes: Vec<CanonicalAthlete> = harness.store.scan(Table::Athletes)?;
            let relays: Vec<RelayResult> = harness.store.scan(Table::RelayResults)?;
            exact_capture_relays(&relays, &athletes)?;
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
                first.rows
                    == u64::try_from(
                        captured_individual_keys()?.len() + captured_relay_parents()?.len()
                    )?,
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
