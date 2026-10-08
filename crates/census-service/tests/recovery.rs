#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "recovery_mod/discovery.rs"]
mod discovery;
#[path = "recovery_mod/interrupted_worker.rs"]
mod interrupted_worker;
#[path = "recovery_mod/jurisdiction_replay.rs"]
mod jurisdiction_replay;
#[path = "recovery_mod/material.rs"]
mod material;
#[path = "recovery_mod/process.rs"]
mod process;
#[path = "recovery_mod/service_lifecycle.rs"]
mod service_lifecycle;
#[path = "recovery_mod/store_replay.rs"]
mod store_replay;
#[path = "recovery_mod/worker_export.rs"]
mod worker_export;

use material::wi_options;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use census_crawl::net::Fetcher;
use census_crawl::{ks, milesplit, AdapterContext, AdapterReport};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, Evidence,
    GradYear, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation};
use census_service::census;
use census_service::census::CollectOptions;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const KS_DIRECTORY_URL: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
const UNIT_PHASE: &str = "recovery_units";
const OBSERVED_ON: &str = "2026-09-20";

const KS_DIRECTORY_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/ks/kshsaa_directory_a.json");
const ATHLETICLIVE_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/athleticlive/meets-sample.csv");
const WI_TEAMS_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_roster_52649.html");
fn wi_teams_phase() -> String {
    census::teams_phase(UsJurisdiction::Wisconsin)
}

fn wi_rosters_phase() -> String {
    let options = wi_options(None);
    census::rosters_phase(
        UsJurisdiction::Wisconsin,
        options.school_year,
        options.revision,
    )
}

const CENSUS_BIN: &str = env!("CARGO_BIN_EXE_census-service");
const SERVE_BIN: &str = env!("CARGO_BIN_EXE_census-serve");

const DEAD_PROXY: &str = "http://127.0.0.1:9";
const DISCOVER_ATTEMPTS: usize = 200;
const DISCOVER_RETRY_DELAY: Duration = Duration::from_millis(25);
const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";

fn note(scenario: &str, message: impl std::fmt::Display) {
    println!("[recovery:{scenario}] {message}");
}

fn digest_of<T: serde::Serialize>(value: &T) -> TestResult<String> {
    let bytes = serde_json::to_vec(value)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn journal_keys(store: &Store, phase: &str) -> TestResult<BTreeSet<String>> {
    Ok(store.journal_keys(phase)?.into_iter().collect())
}

fn table_counts(store: &Store) -> TestResult<BTreeMap<String, u64>> {
    Ok(store.stats()?.tables.into_iter().collect())
}

fn count_of(counts: &BTreeMap<String, u64>, table: Table) -> u64 {
    counts.get(table.file()).copied().map_or(0, |value| value)
}

fn meet_ids(store: &Store) -> TestResult<BTreeSet<String>> {
    Ok(store
        .scan::<CanonicalMeet>(Table::Meets)?
        .into_iter()
        .map(|meet| meet.id.to_string())
        .collect())
}

fn school_ids(store: &Store) -> TestResult<BTreeSet<String>> {
    Ok(store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .map(|school| school.id.to_string())
        .collect())
}

fn open_store(root: &Path) -> TestResult<Store> {
    Ok(Store::open(root)?)
}

fn fetcher_for(store: &Store) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?)
}

fn context<'a>(
    fetcher: &'a Fetcher,
    store: &'a Store,
    observed_on: &str,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::DEFAULT,
        observed_on: observed_on.to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str(observed_on, "%Y-%m-%d")?,
        recording: None,
    })
}
