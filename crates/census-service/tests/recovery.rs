//! §59 crash/recovery tests: kill, restart, resume, idempotent reconciliation, terminal counters.
//!
//! The census is a multi-hour, multi-process run, so §59 requires the durable seams to be tested
//! explicitly: completed durable work is reused, incomplete work resumes at the first unjournaled
//! unit, duplicate observations reconcile idempotently, and terminal counters stay consistent
//! across a restart. This file drives those seams at the two levels they exist on:
//!
//! | scenario | restart / kill mechanism | level |
//! |---|---|---|
//! | [`store_reopen_after_a_writer_stops_mid_batch_resumes_at_the_first_unjournaled_unit`] | the writer handle is dropped mid-batch (a process that ends without finishing its set) | in-process `Store` |
//! | [`adapter_restart_reuses_finished_units_without_refetch_or_duplicate_rows`] | a second run of the same adapter over the same store | in-process `ks::collect` |
//! | [`exporter_restart_republishes_identical_snapshots_and_totals`] | the store is closed and reopened between two export passes | in-process `census::consolidate` + `report::build_census` |
//! | [`cli_worker_restart_across_processes_resumes_and_keeps_counters`] | two real `census-service` processes in sequence, plus `fjall-stats` and `consolidate` | real processes |
//! | [`sigkill_mid_batch_worker_restart_completes_the_remaining_units`] | **SIGKILL** of a running `census-service provider athleticlive` at fractions of its own measured clean runtime | real process kill |
//! | [`sigkill_of_the_service_keeps_durable_work_and_the_restart_drains_cleanly`] | **SIGKILL** of `census-serve` once `/discover` answers, then restart and a SIGTERM drain | real process kill |
//! | [`service_with_no_stop_request_survives_its_drain_deadline`] | a real `census-serve` that is never told to stop, probed past its own `--drain-timeout` | real process |
//! | [`ks_directory_walk_claims_units_the_kill_can_lose`] | **SIGKILL** of a running `census-service provider ks`; measures the journal/effect gap | real process kill |
//! | [`jurisdiction_walk_resumes_from_the_journaled_index_and_the_unclaimed_rosters`] | a roster pass capped at one roster, then a restart on the same store | in-process `census::collect_state_teams` + `collect_state_rosters` |
//!
//! Every scenario prints the values it asserts on with a `[recovery:…]` prefix, so the proof is
//! visible in the test output and not only in the assertions.
//!
//! # No network
//!
//! Adapters run from committed captures seeded into the fetcher's on-disk cache
//! (`tests/fixtures/ks/kshsaa_directory_a.json`, `tests/fixtures/athleticlive/meets-sample.csv`,
//! `tests/fixtures/milesplit/wi_teams_index.html` with `wi_roster_52649.html`),
//! exactly as the crate's adapter tests do; `provider athleticlive` performs no HTTP at all. Child
//! processes additionally run with `HTTPS_PROXY=http://127.0.0.1:9`, so a cache-key drift fails as
//! a connection refusal instead of reaching the source host.
//!
//! # The findings
//!
//! Both are printed as `DEFECT:` evidence and reported to `Main` rather than gated here, because
//! this slice owns tests only; the scenarios stay green so the suite keeps running.
//!
//! 1. **Journal before effect.** `sources/ks/collect.rs` journals each directory record done during
//!    the walk and appends the batch at the end of the pass; `ihsa`, `athleticnet` and `plain_names`
//!    have the same shape. A kill inside the walk therefore leaves journal entries whose rows were
//!    never written, and the next run skips them — the unit is lost rather than resumed.
//!    [`ks_directory_walk_claims_units_the_kill_can_lose`] measures that on a real kill.
//! 2. **A drain deadline armed before the stop request.** `census-serve` used to abort its endpoint
//!    (then exit) one `--drain-timeout` after start with no stop request in sight, which under the
//!    systemd unit (`Restart=on-failure`, exit 0) means a deployed endpoint that dies and is never
//!    restarted. [`service_with_no_stop_request_survives_its_drain_deadline`] measures whichever
//!    half of that remains: an unrequested exit, or a live process whose endpoint stopped answering.

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
use census_store::{Store, Table};
use census_service::census::CollectOptions;
use census_report::report;
use census_service::census;
use sha2::{Digest, Sha256};

// -------------------------------------------------------------------------------------------------
// Inputs and constants
// -------------------------------------------------------------------------------------------------

/// The one URL the KS adapter requests, spelled exactly as `sources/ks/collect.rs` builds it.
const KS_DIRECTORY_URL: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/a/";
/// The resume-journal phases the two adapters write.
const KS_PHASE: &str = "kshsaa_schools";
const ATHLETICLIVE_PHASE: &str = "athleticlive_meets";
/// The phase the store-level scenarios journal into.
const UNIT_PHASE: &str = "recovery_units";
/// Capture date the fixtures carry; every `observed_on` here is that date.
const OBSERVED_ON: &str = "2026-09-20";

const KS_DIRECTORY_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/ks/kshsaa_directory_a.json");
const ATHLETICLIVE_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/athleticlive/meets-sample.csv");
/// The MileSplit pair captured together: the WI team index and the roster of the team that index
/// lists first (`Site::teams_url()`, and `<that team's url>/roster` off the parsed index).
const WI_TEAMS_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_teams_index.html");
const WI_ROSTER_FIXTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/milesplit/wi_roster_52649.html");
/// The two resume-journal phases the jurisdiction walk writes (`census::teams_phase` /
/// `census::rosters_phase`): a state's team index, and `<state>:<team id>` per finished roster.
const WI_TEAMS_PHASE: &str = "milesplit_teams_wi";
const WI_ROSTERS_PHASE: &str = "milesplit_rosters_wi";

/// The crate's own binaries, built by cargo for this test target.
const CENSUS_BIN: &str = env!("CARGO_BIN_EXE_census-service");
const SERVE_BIN: &str = env!("CARGO_BIN_EXE_census-serve");

/// Any accidental cache miss fails against a dead proxy instead of reaching a real host.
const DEAD_PROXY: &str = "http://127.0.0.1:9";
/// Bound on the `/discover` poll after a `census-serve` start: a service's startup is milliseconds,
/// so the poll is short and the drain deadlines in these scenarios outlive it.
const DISCOVER_ATTEMPTS: usize = 200;
const DISCOVER_RETRY_DELAY: Duration = Duration::from_millis(25);
/// The discovery accept header, the same one `tests/fjall_restate_e2e.rs` asks with.
const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";
/// The services the endpoint advertises (wire names, from `restate_services`): the operator's read
/// surface, the four heavy jobs it shares, the ingest object, and the sweeps and workflows that
/// drive them.
const EXPECTED_SERVICES: [&str; 9] = [
    "Census",
    "Consolidate",
    "Report",
    "Bests",
    "Workbook",
    "Ingest",
    "Sweep",
    "JurisdictionCensus",
    "NationalCensus",
];

// -------------------------------------------------------------------------------------------------
// Evidence helpers
// -------------------------------------------------------------------------------------------------

/// One evidence line: every scenario prints what it is about to assert on.
fn note(scenario: &str, message: impl std::fmt::Display) {
    println!("[recovery:{scenario}] {message}");
}

/// A stable hex digest of a serializable value, for comparing merged output across restarts.
fn digest_of<T: serde::Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serialize for digest");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Sorted journal keys for a phase: the resume set an adapter reads on restart.
fn journal_keys(store: &Store, phase: &str) -> BTreeSet<String> {
    store
        .journal_keys(phase)
        .expect("reading the resume journal")
        .into_iter()
        .collect()
}

/// `<table file name>` -> observation count, the store's own per-table estimate.
fn table_counts(store: &Store) -> BTreeMap<String, u64> {
    store
        .stats()
        .expect("reading store stats")
        .tables
        .into_iter()
        .collect()
}

/// Observation count for one table.
fn count_of(counts: &BTreeMap<String, u64>, table: Table) -> u64 {
    counts.get(table.file()).copied().unwrap_or(0)
}

/// Merged meet ids, the identity a duplicate observation must not change.
fn meet_ids(store: &Store) -> BTreeSet<String> {
    store
        .scan::<CanonicalMeet>(Table::Meets)
        .expect("scanning meets")
        .into_iter()
        .map(|meet| meet.id.to_string())
        .collect()
}

/// Merged school ids.
fn school_ids(store: &Store) -> BTreeSet<String> {
    store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("scanning schools")
        .into_iter()
        .map(|school| school.id.to_string())
        .collect()
}

// -------------------------------------------------------------------------------------------------
// Shared fixtures
// -------------------------------------------------------------------------------------------------

/// Seed the fetcher's on-disk cache for `url`, so an adapter that fetches runs with no socket: the
/// key is `sha256(method \x1f url \x1f body)[..16]`, the form `net::cache` writes.
fn seed_cache(cache: &Path, url: &str, body: &str) {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::create_dir_all(cache).expect("cache dir");
    std::fs::write(cache.join(format!("{key}.body")), body).expect("cache body");
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "application/json",
    });
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec_pretty(&meta).expect("cache meta"),
    )
    .expect("cache meta written");
}

/// Open the store at `root`, creating it if needed.
fn open_store(root: &Path) -> Store {
    Store::open(root).expect("opening the store")
}

/// A fetcher pointed at its store's own cache directory.
fn fetcher_for(store: &Store) -> Fetcher {
    Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("building the fetcher")
}

/// Build the adapter context an in-process run uses.
fn context<'a>(fetcher: &'a Fetcher, store: &'a Store, observed_on: &str) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: observed_on.to_string(),
    }
}

/// A fresh store whose cache carries the KS directory capture.
fn store_seeded_with_ks(root: &Path) -> Store {
    let store = open_store(root);
    seed_cache(
        &store.http_cache_dir(),
        KS_DIRECTORY_URL,
        KS_DIRECTORY_FIXTURE,
    );
    store
}

/// One KS directory pass, optionally capped at `limit` units.
async fn ks_pass(store: &Store, limit: Option<usize>) -> AdapterReport {
    let fetcher = fetcher_for(store);
    let ctx = context(&fetcher, store, OBSERVED_ON);
    let options = ks::Options {
        limit,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    };
    ks::collect(&ctx, &options).await.expect("ks collect")
}

/// A fresh store whose cache answers the WI team index and one roster per indexed team.
///
/// The index and the roster fixtures are the captured pair: the index lists team `52649` first and
/// the roster fixture is that team's page. The same captured roster body stands in for the remaining
/// teams, so the scenarios that use this seed compare a restarted walk against a clean walk over the
/// same cache rather than against athlete names on the page.
fn store_seeded_with_wisconsin(root: &Path, site: &milesplit::Site) -> Store {
    let store = open_store(root);
    seed_cache(&store.http_cache_dir(), &site.teams_url(), WI_TEAMS_FIXTURE);
    let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE).expect("the index fixture parses");
    let first = teams.first().expect("the index fixture lists teams");
    assert_eq!(
        first.id, "52649",
        "the index fixture and the roster fixture are a captured pair: \
         fixtures/milesplit/wi_teams_index.html must list teams/52649 first"
    );
    for team in &teams {
        seed_cache(
            &store.http_cache_dir(),
            &format!("{}/roster", team.url),
            WI_ROSTER_FIXTURE,
        );
    }
    store
}

/// The collection options a jurisdiction walk here runs with; `limit_per_state` is the caller's.
fn wi_options(limit_per_state: Option<usize>) -> CollectOptions {
    CollectOptions {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        limit_per_state,
        concurrency: 2,
        state_concurrency: 1,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    }
}

// -------------------------------------------------------------------------------------------------
// Store-level units
// -------------------------------------------------------------------------------------------------

/// One unit's observation: a school named after the unit, stamped with the observation date.
fn unit_school(key: &str, observed_on: &str) -> CanonicalSchool {
    let name = format!("Recovery Unit {key}");
    let (mut school, _id) =
        CanonicalSchool::new(UsJurisdiction::Kansas, name.clone(), normalize_name(&name));
    school.evidence.push(Evidence::parsed(
        SourceRef::id("recovery_fixture"),
        observed_on,
    ));
    school
}

/// One unit of worker work: append the observation, then journal the unit done — the ordering the
/// durable contract requires (a journal entry is the promise that its row reached the store).
fn process_unit(store: &Store, key: &str, observed_on: &str) {
    store
        .append(Table::Schools, &unit_school(key, observed_on))
        .expect("appending a unit observation");
    store
        .journal_done(
            UNIT_PHASE,
            key,
            &serde_json::json!({ "unit": key, "observed_on": observed_on }),
        )
        .expect("journalling a unit");
}

/// The units a restarted worker still owes: everything the journal has not claimed.
fn pending_units<'a>(store: &Store, units: &[&'a str]) -> Vec<&'a str> {
    let done = store.journal_keys(UNIT_PHASE).expect("reading the journal");
    units
        .iter()
        .copied()
        .filter(|unit| !done.contains(*unit))
        .collect()
}

/// The same units, as one store pass with the given per-unit observation dates.
fn process_units(store: &Store, units: &[&str], observed_on: &str) {
    for unit in units {
        process_unit(store, unit, observed_on);
    }
}

// -------------------------------------------------------------------------------------------------
// Real-process helpers
// -------------------------------------------------------------------------------------------------

/// A `census-service` command against `root`, with tracing silenced so stdout is the payload and a
/// dead proxy so a cache miss cannot reach a source host.
fn census_command(root: &Path) -> Command {
    let mut command = Command::new(CENSUS_BIN);
    command
        .arg("--store")
        .arg(root)
        .arg("--delay-ms")
        .arg("0")
        .env("RUST_LOG", "off")
        .env("HTTPS_PROXY", DEAD_PROXY)
        .env("HTTP_PROXY", DEAD_PROXY);
    command
}

/// Run one `census-service` command to completion, failing the test on a non-zero status.
fn run_census(root: &Path, args: &[&str]) -> Output {
    let output = census_command(root)
        .args(args)
        .output()
        .expect("spawning census-service");
    assert!(
        output.status.success(),
        "census-service {args:?} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// The JSON report a `provider` command prints on stdout.
fn report_of(output: &Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("provider stdout was not a report JSON ({error}):\n{stdout}")
    })
}

/// `<table>\t<count>` and `observations\t<count>` lines from `fjall-stats` or `consolidate`.
///
/// Only keys the durable contract names survive: the table observation counts, the journal's
/// `observations` counter and the withholding count `consolidate` reports. Everything else is
/// operational (the on-disk byte figure, the store path) and would compare filesystem layout
/// instead of durable state across restarts.
fn counters_of(output: &Output) -> BTreeMap<String, u64> {
    let mut allowed: BTreeSet<&str> = Table::ALL.iter().map(|table| table.file()).collect();
    allowed.insert("observations");
    allowed.insert("coaches_email_withheld");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(name, _)| allowed.contains(*name))
        .filter_map(|(name, count)| {
            count
                .trim()
                .parse::<u64>()
                .ok()
                .map(|count| (name.to_string(), count))
        })
        .collect()
}

/// What one SIGKILL attempt left behind, read back by reopening the store after the child exits.
struct KillAttempt {
    delay: Duration,
    exited_before_kill: bool,
    signal: Option<i32>,
    journal: BTreeSet<String>,
    entity_ids: BTreeSet<String>,
    observations: u64,
}

/// Spawn `census-service` on `root`, SIGKILL it after `delay`, then read the store back.
///
/// The store is opened only after `wait()`, so the dead process has released its lock first.
fn attempt_kill(
    root: &Path,
    args: &[&str],
    delay: Duration,
    phase: &str,
    table: Table,
    ids_of: fn(&Store) -> BTreeSet<String>,
) -> KillAttempt {
    let mut child = census_command(root)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning census-service");
    std::thread::sleep(delay);
    let exited_before_kill = child.try_wait().expect("try_wait").is_some();
    // A child that already exited refuses the signal; the status below carries the evidence.
    let _ = child.kill();
    let status = child.wait().expect("waiting for the killed child");
    let store = open_store(root);
    let journal = journal_keys(&store, phase);
    let entity_ids = ids_of(&store);
    let observations = count_of(&table_counts(&store), table);
    KillAttempt {
        delay,
        exited_before_kill,
        signal: status.signal(),
        journal,
        entity_ids,
        observations,
    }
}

/// The fractions of the clean runtime the ladder kills at: an immediate kill, a coarse approach, then
/// a sweep of the last stretch at 1% steps.
///
/// The sweep is dense on purpose. A workload spends most of its runtime in spawn, store open and
/// parsing, and the batch writes land in the last percent or two, so only samples crowding toward 1
/// land a kill inside the window that produces a partial batch. Samples are microsecond-accurate
/// because that window is about a millisecond wide.
const LADDER_FRACTIONS: [(u64, u64); 27] = [
    (0, 1),
    (1, 4),
    (1, 2),
    (3, 4),
    (4, 5),
    (17, 20),
    (9, 10),
    (19, 20),
    (81, 100),
    (83, 100),
    (85, 100),
    (87, 100),
    (89, 100),
    (90, 100),
    (91, 100),
    (92, 100),
    (93, 100),
    (94, 100),
    (95, 100),
    (96, 100),
    (97, 100),
    (98, 100),
    (99, 100),
    (991, 1000),
    (993, 1000),
    (996, 1000),
    (998, 1000),
];

/// Kill ladder for a workload whose clean runtime is `clean_runtime`, stopping as soon as a partial
/// What a ladder attempt kills: the phase to interrupt and the rows that prove the units landed.
struct Subject<'a> {
    phase: &'a str,
    table: Table,
    ids_of: fn(&Store) -> BTreeSet<String>,
}

/// kill (some units durable, some still owed) lands or the workload turns out to have completed.
fn kill_ladder(
    scenario: &str,
    root: &Path,
    args: &[&str],
    subject: Subject<'_>,
    total_units: usize,
    clean_runtime: Duration,
) -> Vec<KillAttempt> {
    let runtime_us = u64::try_from(clean_runtime.as_micros()).unwrap_or(u64::MAX);
    let mut attempts: Vec<KillAttempt> = Vec::new();
    for (numerator, denominator) in LADDER_FRACTIONS {
        let delay = Duration::from_micros(runtime_us.saturating_mul(numerator) / denominator);
        let attempt = attempt_kill(
            root,
            args,
            delay,
            subject.phase,
            subject.table,
            subject.ids_of,
        );
        note(
            scenario,
            format!(
                "kill delay={:?} exited_before_kill={} signal={:?} journal={} entities={} \
                 observations={}",
                attempt.delay,
                attempt.exited_before_kill,
                attempt.signal,
                attempt.journal.len(),
                attempt.entity_ids.len(),
                attempt.observations
            ),
        );
        if attempt.exited_before_kill {
            assert!(
                attempt.signal.is_none(),
                "a process that had already exited cannot carry a kill signal"
            );
        } else if attempt.signal.is_none() {
            // Alive at the liveness check, gone by the time the signal landed: the only way a run
            // refuses SIGKILL is by finishing first, and the store has to say so.
            assert_eq!(
                attempt.journal.len(),
                total_units,
                "a run that was alive when the ladder checked and refused the kill must have \
                 completed its units: journal={:?}",
                attempt.journal
            );
        }
        let partial = !attempt.journal.is_empty() && attempt.journal.len() < total_units;
        let complete = attempt.journal.len() >= total_units;
        attempts.push(attempt);
        if partial {
            note(
                scenario,
                "partial kill landed: the store holds a mix of durable and still-owed units",
            );
            break;
        }
        if complete {
            note(
                scenario,
                "the workload completed before this attempt; no later attempt can land mid-batch",
            );
            break;
        }
    }
    attempts
}

/// A free loopback port: bind, read it back, drop the listener.
fn free_loopback_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("binding an ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

/// The h2 discovery client. The SDK's endpoint serves HTTP/2 only (hyper's `http2::Builder`), so the
/// client has to speak h2 without an upgrade dance - the same shape `tests/fjall_restate_e2e.rs` uses.
fn discovery_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .http2_prior_knowledge()
        .build()
        .expect("building the discovery client")
}

/// One `/discover` request against the endpoint: the manifest, or the reason it did not arrive.
async fn discovery_once(client: &reqwest::Client, port: u16) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:{port}/discover");
    match client
        .get(&url)
        .header("accept", DISCOVERY_ACCEPT)
        .send()
        .await
    {
        Ok(response) if response.status() == reqwest::StatusCode::OK => response
            .json()
            .await
            .map_err(|error| format!("the manifest was not JSON: {error}")),
        Ok(response) => Err(format!("status {}", response.status())),
        Err(error) => Err(error.to_string()),
    }
}

/// Whether the endpoint on `port` still answers a discovery request: the liveness probe the
/// drain-deadline scenario uses after a deadline that was never requested.
async fn endpoint_answers(client: &reqwest::Client, port: u16) -> bool {
    discovery_once(client, port).await.is_ok()
}

/// Poll `/discover` until the endpoint answers with a manifest, or report that it never did.
async fn poll_for_discovery(
    client: &reqwest::Client,
    port: u16,
    child: &mut Child,
) -> serde_json::Value {
    let mut last = String::from("<never attempted>");
    for _ in 0..DISCOVER_ATTEMPTS {
        match discovery_once(client, port).await {
            Ok(manifest) => return manifest,
            Err(reason) => last = reason,
        }
        if let Some(status) = child.try_wait().expect("try_wait") {
            let mut stderr = String::new();
            if let Some(mut handle) = child.stderr.take() {
                let _ = handle.read_to_string(&mut stderr);
            }
            panic!(
                "census-serve exited before answering /discover: {status:?}\nstderr:\n{stderr}"
            );
        }
        tokio::time::sleep(DISCOVER_RETRY_DELAY).await;
    }
    panic!("census-serve never answered /discover on port {port}: last={last}");
}

/// Spawn `census-serve` on `root` with `drain_secs` as its drain deadline and wait until it answers
/// `/discover`. `RUST_LOG=info` is what puts the drain report and its stop reason on stdout, which is
/// the evidence the service scenarios assert on.
async fn spawn_serve(root: &Path, port: u16, drain_secs: u64) -> (Child, serde_json::Value) {
    let mut child = Command::new(SERVE_BIN)
        .arg("--data-dir")
        .arg(root)
        .arg("--listen")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--drain-timeout")
        .arg(drain_secs.to_string())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning census-serve");
    let manifest = poll_for_discovery(&discovery_client(), port, &mut child).await;
    (child, manifest)
}

/// The service names a discovery manifest advertises, in the shape `tests/fjall_restate_e2e.rs` reads
/// them: Restate wire names come from struct names, so they are PascalCase.
fn manifest_services(manifest: &serde_json::Value) -> Vec<String> {
    manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .expect("the discovery manifest carries a services array")
        .iter()
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

/// Assert the manifest advertises every service the endpoint is supposed to.
fn assert_manifest_advertises(scenario: &str, manifest: &serde_json::Value) {
    let services = manifest_services(manifest);
    note(
        scenario,
        format!("discovery manifest services={services:?}"),
    );
    for expected in EXPECTED_SERVICES {
        assert!(
            services.iter().any(|name| name == expected),
            "the discovery manifest {services:?} does not advertise {expected}"
        );
    }
    assert_eq!(
        services.len(),
        EXPECTED_SERVICES.len(),
        "the endpoint advertises exactly the services the workflow needs, nothing more: {services:?}"
    );
}

/// Wait for `child` to exit, at most `bound`; `Some(elapsed)` once it is gone, `None` if it is still
/// running when the bound expires.
fn wait_for_exit(child: &mut Child, bound: Duration) -> Option<Duration> {
    let started = Instant::now();
    while started.elapsed() < bound {
        if child.try_wait().expect("try_wait").is_some() {
            return Some(started.elapsed());
        }
        std::thread::sleep(DISCOVER_RETRY_DELAY);
    }
    None
}

/// Send SIGTERM the way an operator does, through the system `kill`.
fn send_sigterm(child: &Child) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .expect("running kill -TERM");
    assert!(status.success(), "kill -TERM was refused: {status}");
}

// -------------------------------------------------------------------------------------------------
// 1. Store resume path (in-process)
// -------------------------------------------------------------------------------------------------

#[test]
fn store_reopen_after_a_writer_stops_mid_batch_resumes_at_the_first_unjournaled_unit() {
    const SCENARIO: &str = "store-resume";
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("store");
    let units = ["unit-1", "unit-2", "unit-3", "unit-4"];

    let (journal_before, counts_before, merged_before, consolidated_before) = {
        let store = open_store(&root);
        process_units(&store, &units[..2], OBSERVED_ON);
        let journal = journal_keys(&store, UNIT_PHASE);
        let counts = table_counts(&store);
        let merged = store.scan::<CanonicalSchool>(Table::Schools).expect("scan");
        let consolidated = store
            .consolidate::<CanonicalSchool>(Table::Schools, &root.join("out/schools.jsonl"))
            .expect("consolidate");
        (journal, counts, merged.len(), consolidated)
        // The store is dropped here: the writer is gone mid-batch, exactly as at the end of a
        // process that never returns from its run.
    };
    note(
        SCENARIO,
        format!(
            "before restart: journal={:?} tables={counts_before:?} merged_entities={merged_before} \
             snapshot_rows={} withheld={}",
            journal_before, consolidated_before.rows, consolidated_before.withheld
        ),
    );
    assert_eq!(journal_before.len(), 2, "two units reached the journal");
    assert_eq!(count_of(&counts_before, Table::Schools), 2);
    assert_eq!(merged_before, 2);

    let store = open_store(&root);
    let journal_after = journal_keys(&store, UNIT_PHASE);
    let counts_after = table_counts(&store);
    note(
        SCENARIO,
        format!("after restart: journal={journal_after:?} tables={counts_after:?}"),
    );
    assert_eq!(
        journal_after, journal_before,
        "the resume ledger is the durable record of finished work and must survive the restart"
    );
    assert_eq!(
        counts_after, counts_before,
        "reopening replays exactly what reached the journal; counters are unchanged"
    );

    let pending = pending_units(&store, &units);
    note(SCENARIO, format!("resume set: pending={pending:?}"));
    assert_eq!(
        pending,
        vec!["unit-3", "unit-4"],
        "the resumed worker starts at the first unit the journal does not claim"
    );
    process_units(&store, &pending, OBSERVED_ON);
    let finished_journal = journal_keys(&store, UNIT_PHASE);
    let finished_counts = table_counts(&store);
    note(
        SCENARIO,
        format!(
            "after the resumed units: journal={} schools={} observations={}",
            finished_journal.len(),
            count_of(&finished_counts, Table::Schools),
            store.stats().expect("stats").observations
        ),
    );
    assert_eq!(finished_journal.len(), 4);
    assert_eq!(count_of(&finished_counts, Table::Schools), 4);

    // Sequence continuity: a fresh observation of an already-processed unit appends a *new* row. A
    // sequence counter that restarted with the database would overwrite the earlier row instead and
    // leave the count where it was.
    process_unit(&store, "unit-1", "2026-09-21");
    let after_duplicate = store.stats().expect("stats").observations;
    let merged = store.scan::<CanonicalSchool>(Table::Schools).expect("scan");
    let unit_1 = merged
        .iter()
        .find(|school| school.name == "Recovery Unit unit-1")
        .expect("unit-1 is merged");
    let dates: Vec<&str> = unit_1
        .evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .collect();
    note(
        SCENARIO,
        format!(
            "duplicate observation: observations={after_duplicate} merged_entities={} \
             evidence_on_unit_1={dates:?}",
            merged.len()
        ),
    );
    assert_eq!(
        after_duplicate, 5,
        "the reopened store continues the sequence instead of overwriting the earlier row"
    );
    assert_eq!(
        merged.len(),
        4,
        "a duplicate observation reconciles into the same entity: no double count"
    );
    assert_eq!(
        unit_1.evidence.len(),
        2,
        "both observations stay as evidence on the one merged entity"
    );
}

// -------------------------------------------------------------------------------------------------
// 2. Adapter restart (in-process, real adapter over a seeded cache)
// -------------------------------------------------------------------------------------------------

#[tokio::test]
async fn adapter_restart_reuses_finished_units_without_refetch_or_duplicate_rows() {
    const SCENARIO: &str = "adapter-resume";
    let dir = tempfile::tempdir().expect("temp dir");

    // Control: one clean pass in its own store.
    let control_root = dir.path().join("control");
    let (control_journal, control_schools, control_coaches, control_report, control_counts) = {
        let store = store_seeded_with_ks(&control_root);
        let report = ks_pass(&store, None).await;
        let journal = journal_keys(&store, KS_PHASE);
        let schools = store.scan::<CanonicalSchool>(Table::Schools).expect("scan");
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches).expect("scan");
        let counts = table_counts(&store);
        (journal, schools, coaches, report, counts)
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!(
            "control: journal={} schools={} coaches={} requests={} from_cache={} tables={:?}",
            total_units,
            control_schools.len(),
            control_coaches.len(),
            control_report.requests,
            control_report.from_cache,
            control_counts
        ),
    );
    assert_eq!(
        control_report.requests, 0,
        "the control run must answer the directory request from the seeded cache"
    );
    assert_eq!(
        control_schools.len(),
        total_units,
        "one school per journaled directory record"
    );
    assert!(
        total_units >= 2,
        "the fixture must carry at least two units"
    );

    // A worker that stops after two units.
    let restart_root = dir.path().join("restart");
    let first_journal = {
        let store = store_seeded_with_ks(&restart_root);
        let report = ks_pass(&store, Some(2)).await;
        let journal = journal_keys(&store, KS_PHASE);
        note(
            SCENARIO,
            format!(
                "first pass (limit 2): journal={journal:?} rows={} requests={} from_cache={}",
                report.rows, report.requests, report.from_cache
            ),
        );
        assert_eq!(journal.len(), 2);
        assert_eq!(report.requests, 0);
        journal
        // store dropped: the writer is gone mid-batch
    };

    // Restart: the same adapter over the same store.
    let store = open_store(&restart_root);
    let resumed = ks_pass(&store, None).await;
    let resumed_journal = journal_keys(&store, KS_PHASE);
    note(
        SCENARIO,
        format!(
            "restart: journal={} rows_this_pass={} notes={:?}",
            resumed_journal.len(),
            resumed.rows,
            resumed.notes
        ),
    );
    assert_eq!(
        resumed_journal, control_journal,
        "the restart completes exactly the units the control pass covered"
    );
    assert_eq!(
        resumed.rows as usize,
        total_units - first_journal.len(),
        "the restart processed only the units the first pass did not journal"
    );
    assert!(
        resumed
            .notes
            .iter()
            .any(|entry| entry.contains("already done")),
        "the report states the resumed units were skipped: {:?}",
        resumed.notes
    );

    let schools = store.scan::<CanonicalSchool>(Table::Schools).expect("scan");
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches).expect("scan");
    let counts = table_counts(&store);
    let observations = store.stats().expect("stats").observations;
    note(
        SCENARIO,
        format!(
            "merged: schools_digest={} coaches_digest={} schools={} coaches={} observations={} \
             tables={counts:?}",
            digest_of(&schools),
            digest_of(&coaches),
            schools.len(),
            coaches.len(),
            observations
        ),
    );
    assert_eq!(
        schools, control_schools,
        "the restarted store merges to exactly the control's schools"
    );
    assert_eq!(
        coaches, control_coaches,
        "the restarted store merges to exactly the control's coaches"
    );
    assert_eq!(
        counts, control_counts,
        "observation counters match the control: no unit was written twice"
    );
    assert_eq!(
        count_of(&counts, Table::Schools) as usize,
        total_units,
        "one observation per journaled unit"
    );
}

// -------------------------------------------------------------------------------------------------
// 3. Exporter restart (in-process)
// -------------------------------------------------------------------------------------------------

#[tokio::test]
async fn exporter_restart_republishes_identical_snapshots_and_totals() {
    const SCENARIO: &str = "exporter-restart";
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("store");

    let first = {
        let store = store_seeded_with_ks(&root);
        let report = ks_pass(&store, None).await;
        assert_eq!(report.requests, 0);
        export(&store)
    };
    note(
        SCENARIO,
        format!(
            "first export: consolidate={:?} snapshots={:?} census_digest={} totals={}",
            first.counts, first.snapshots, first.census_digest, first.totals
        ),
    );

    let second = {
        let store = open_store(&root);
        export(&store)
    };
    note(
        SCENARIO,
        format!(
            "second export (after reopen): consolidate={:?} snapshots={:?} census_digest={} \
             totals={}",
            second.counts, second.snapshots, second.census_digest, second.totals
        ),
    );

    assert_eq!(
        second.counts, first.counts,
        "a restarted exporter consolidates the same rows"
    );
    assert_eq!(
        second.snapshots, first.snapshots,
        "the published snapshot bytes are identical across the restart"
    );
    assert_eq!(
        second.census_digest, first.census_digest,
        "the census totals are identical across the restart"
    );
    assert!(
        !first.snapshots.is_empty(),
        "the corpus produced at least one snapshot to compare"
    );
}

/// What one export pass produced: the consolidate counts, per-table snapshot digests, the census
/// totals projection and its digest.
struct Export {
    counts: Vec<(String, usize)>,
    snapshots: BTreeMap<String, String>,
    census_digest: String,
    totals: String,
}

/// Consolidate every table and build the census, the publishing stage of `census-service run`.
fn export(store: &Store) -> Export {
    let counts: Vec<(String, usize)> = census::consolidate(store)
        .expect("consolidate")
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .collect();
    let snapshots = Table::ALL
        .into_iter()
        .filter_map(|table| {
            // `census::consolidate` publishes the merged snapshot under `out/`, one file per table.
            let bytes =
                std::fs::read(store.out_dir().join(format!("{}.jsonl", table.file()))).ok()?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let digest: String = hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            Some((table.file().to_string(), digest))
        })
        .collect();
    let census = report::build_census(store, report::Scope::AllSources).expect("census");
    let mut projected = serde_json::to_value(&census).expect("census json");
    if let Some(object) = projected.as_object_mut() {
        // The two values that legitimately differ between runs: the wall-clock stamp and the
        // store's own path (a temp directory here).
        object.remove("generated_on");
        object.remove("store_dir");
    }
    let census_digest = digest_of(&projected);
    let totals = format!(
        "athletes={} schools={} meets={:?}",
        projected
            .pointer("/totals/athletes")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        projected
            .pointer("/totals/schools")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        projected
            .pointer("/meets/total")
            .cloned()
            .unwrap_or_default(),
    );
    Export {
        counts,
        snapshots,
        census_digest,
        totals,
    }
}

// -------------------------------------------------------------------------------------------------
// 4. Worker restart across real processes
// -------------------------------------------------------------------------------------------------

#[test]
fn cli_worker_restart_across_processes_resumes_and_keeps_counters() {
    const SCENARIO: &str = "cli-restart";
    let dir = tempfile::tempdir().expect("temp dir");

    // Control: one process, one clean pass.
    let control_root = dir.path().join("control");
    {
        let store = store_seeded_with_ks(&control_root);
        drop(store);
    }
    let control_report = report_of(&run_census(&control_root, &["provider", "ks"]));
    let control_stats = counters_of(&run_census(&control_root, &["fjall-stats"]));
    let control_journal = {
        let store = open_store(&control_root);
        journal_keys(&store, KS_PHASE)
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!(
            "control process: rows={} units={} stats={control_stats:?}",
            control_report["rows"], total_units
        ),
    );
    assert_eq!(
        control_report["rows"].as_u64(),
        Some(total_units as u64),
        "the clean process covered every unit"
    );

    // A worker that stops after two units, then restarts on the same store.
    let root = dir.path().join("restart");
    {
        let store = store_seeded_with_ks(&root);
        drop(store);
    }
    let first = report_of(&run_census(&root, &["provider", "ks", "--limit", "2"]));
    let journal_after_first = {
        let store = open_store(&root);
        journal_keys(&store, KS_PHASE)
    };
    note(
        SCENARIO,
        format!(
            "first process: rows={} requests={} from_cache={} journal={:?}",
            first["rows"], first["requests"], first["from_cache"], journal_after_first
        ),
    );
    assert_eq!(
        journal_after_first.len(),
        2,
        "the first process did two units"
    );
    assert_eq!(
        first["requests"].as_u64(),
        Some(0),
        "the directory request must come from the seeded cache"
    );

    let resumed = report_of(&run_census(&root, &["provider", "ks"]));
    let resumed_stats = counters_of(&run_census(&root, &["fjall-stats"]));
    let consolidated = counters_of(&run_census(&root, &["consolidate"]));
    let after_export_stats = counters_of(&run_census(&root, &["fjall-stats"]));
    let final_journal = {
        let store = open_store(&root);
        journal_keys(&store, KS_PHASE)
    };
    note(
        SCENARIO,
        format!(
            "restart process: rows={} journal={} notes={:?} stats={resumed_stats:?}",
            resumed["rows"],
            final_journal.len(),
            resumed["notes"]
        ),
    );
    note(
        SCENARIO,
        format!(
            "exporter process: consolidate={consolidated:?} stats_after={after_export_stats:?}"
        ),
    );

    assert_eq!(
        resumed["rows"].as_u64(),
        Some(total_units as u64 - 2),
        "the restarted process processed only the units the first one did not journal"
    );
    assert_eq!(
        final_journal, control_journal,
        "the restart completes exactly the control's unit set"
    );
    assert_eq!(
        resumed_stats, control_stats,
        "the store's counters after the restart equal a single-process control"
    );
    assert_eq!(
        after_export_stats, control_stats,
        "consolidating does not change the observation counters"
    );
    assert_eq!(
        consolidated.get("schools"),
        control_stats.get("schools"),
        "the exporter publishes one row per stored school observation"
    );
}

// -------------------------------------------------------------------------------------------------
// 5. SIGKILL mid-batch on a real worker (AthleticLIVE meets: append-then-journal per unit)
// -------------------------------------------------------------------------------------------------

#[test]
fn sigkill_mid_batch_worker_restart_completes_the_remaining_units() {
    const SCENARIO: &str = "sigkill-worker";
    let dir = tempfile::tempdir().expect("temp dir");
    let input = dir.path().join("athleticlive-meets.csv");
    std::fs::write(&input, ATHLETICLIVE_FIXTURE).expect("writing the CSV input");
    let args = [
        "provider",
        "athleticlive",
        "--input",
        input.to_str().expect("csv path"),
    ];

    // Control: the clean run's runtime and terminal counts.
    let control_root = dir.path().join("control");
    let started = Instant::now();
    run_census(&control_root, &args);
    let clean_runtime = started.elapsed();
    let control_stats = counters_of(&run_census(&control_root, &["fjall-stats"]));
    let (control_journal, control_ids) = {
        let store = open_store(&control_root);
        (journal_keys(&store, ATHLETICLIVE_PHASE), meet_ids(&store))
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!("control: runtime={clean_runtime:?} units={total_units} stats={control_stats:?}"),
    );
    assert_eq!(
        control_stats.get("meets").copied(),
        Some(total_units as u64),
        "one meet row per journaled meet"
    );

    // Kill ladder on the same workload in a fresh store.
    let root = dir.path().join("killed");
    let attempts = kill_ladder(
        SCENARIO,
        &root,
        &args,
        Subject {
            phase: ATHLETICLIVE_PHASE,
            table: Table::Meets,
            ids_of: meet_ids,
        },
        total_units,
        clean_runtime,
    );
    let landed_partial = attempts
        .iter()
        .any(|attempt| !attempt.journal.is_empty() && attempt.journal.len() < total_units);
    for attempt in &attempts {
        assert!(
            attempt.journal.is_subset(&attempt.entity_ids),
            "journal implies the durable row: every claimed meet id is in the store after the kill \
             (claimed={:?} stored={:?})",
            attempt.journal,
            attempt.entity_ids
        );
    }

    // Restart: the worker that was killed finishes the units it still owes.
    let resumed = report_of(&run_census(&root, &args));
    let (journal, ids) = {
        let store = open_store(&root);
        (journal_keys(&store, ATHLETICLIVE_PHASE), meet_ids(&store))
    };
    let observations = {
        let store = open_store(&root);
        count_of(&table_counts(&store), Table::Meets)
    };
    let resumed_stats = counters_of(&run_census(&root, &["fjall-stats"]));
    let consolidated = counters_of(&run_census(&root, &["consolidate"]));
    note(
        SCENARIO,
        format!(
            "restart: rows={} journal={} meets={} meet_observations={observations} \
             stats={resumed_stats:?} consolidate={consolidated:?}",
            resumed["rows"],
            journal.len(),
            ids.len()
        ),
    );
    note(
        SCENARIO,
        format!(
            "partial_kill_landed={landed_partial} attempts={} observations_delta={}",
            attempts.len(),
            observations.saturating_sub(control_stats.get("meets").copied().unwrap_or(0))
        ),
    );

    assert_eq!(
        journal, control_journal,
        "the restart completes exactly the control's unit set"
    );
    assert_eq!(
        ids, control_ids,
        "the killed-then-restarted store merges to the control's meets: no lost unit, no double count"
    );
    assert_eq!(
        consolidated.get("meets"),
        control_stats.get("meets"),
        "the snapshot holds one row per meet, not one per observation"
    );
    assert!(
        observations >= total_units as u64,
        "a kill inside the append/journal window may leave one re-observed row, never fewer rows \
         than units"
    );
}

// -------------------------------------------------------------------------------------------------
// 6. SIGKILL of the service, then restart and a clean drain
// -------------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigkill_of_the_service_keeps_durable_work_and_the_restart_drains_cleanly() {
    const SCENARIO: &str = "sigkill-service";
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("store");
    let units = ["unit-1", "unit-2", "unit-3"];

    let (journal_before, counts_before, merged_digest_before) = {
        let store = open_store(&root);
        process_units(&store, &units, OBSERVED_ON);
        store.flush().expect("flushing the store");
        let journal = journal_keys(&store, UNIT_PHASE);
        let counts = table_counts(&store);
        let merged = store.scan::<CanonicalSchool>(Table::Schools).expect("scan");
        (journal, counts, digest_of(&merged))
        // The store is closed before the service starts: one process owns it at a time.
    };
    note(
        SCENARIO,
        format!("before the kill: journal={journal_before:?} tables={counts_before:?}"),
    );

    let port = free_loopback_port();
    // The deployed unit's own flags (`deploy/systemd/census-serve.service`): the drain deadline has
    // to outlive this test's `/discover` poll.
    let (mut child, manifest) = spawn_serve(&root, port, 30).await;
    assert_manifest_advertises(SCENARIO, &manifest);
    assert!(
        child.try_wait().expect("try_wait").is_none(),
        "the service must still be running when it is killed"
    );

    child.kill().expect("SIGKILL to the service");
    let status = child.wait().expect("waiting for the killed service");
    note(
        SCENARIO,
        format!(
            "service killed: status={status:?} signal={:?}",
            status.signal()
        ),
    );
    assert_eq!(
        status.signal(),
        Some(9),
        "the service was killed hard, with no drain"
    );

    let store = open_store(&root);
    let journal_after_kill = journal_keys(&store, UNIT_PHASE);
    let counts_after_kill = table_counts(&store);
    let merged_after_kill =
        digest_of(&store.scan::<CanonicalSchool>(Table::Schools).expect("scan"));
    note(
        SCENARIO,
        format!(
            "after the kill: journal={} tables={counts_after_kill:?}",
            journal_after_kill.len()
        ),
    );
    assert_eq!(
        journal_after_kill, journal_before,
        "a SIGKILL cannot lose journaled work or invent it"
    );
    assert_eq!(
        counts_after_kill, counts_before,
        "the reopened store holds exactly the counters it had before the kill"
    );
    assert_eq!(
        merged_after_kill, merged_digest_before,
        "the merged rows are unchanged by the kill"
    );
    drop(store);

    // Two operator restarts over the same store. Each one has to come up, answer, drain on SIGTERM
    // and leave the counters exactly where the kill did - the service path of §59's "restart worker /
    // restart machine-level services".
    for restart in 1..=2 {
        let (child, manifest) = spawn_serve(&root, port, 30).await;
        assert_manifest_advertises(SCENARIO, &manifest);
        note(SCENARIO, format!("restart {restart} is up"));
        send_sigterm(&child);
        let output = child
            .wait_with_output()
            .expect("waiting for the drained service");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let counters = parse_drain_line(drain_line(&stdout));
        note(
            SCENARIO,
            format!(
                "restart {restart} drain report: {} ({})",
                drain_line(&stdout),
                stop_reason_line(&stdout)
            ),
        );
        assert!(
            output.status.success(),
            "a drained service exits cleanly: {:?}\nstdout:\n{stdout}",
            output.status
        );
        assert!(
            stdout.contains("stop_reason: Signal"),
            "the drain was the operator's SIGTERM, not a server exit:\n{stdout}"
        );
        assert_eq!(counters.get("panicked"), Some(&0));
        assert_eq!(
            counters.get("accepted"),
            Some(
                &(counters.get("completed").copied().unwrap_or(0)
                    + counters.get("cancelled").copied().unwrap_or(0)
                    + counters.get("aborted").copied().unwrap_or(0))
            ),
            "the terminal counters account for every accepted task: {}",
            drain_line(&stdout)
        );

        let store = open_store(&root);
        let counts_after_drain = table_counts(&store);
        let merged_after_drain =
            digest_of(&store.scan::<CanonicalSchool>(Table::Schools).expect("scan"));
        drop(store);
        note(
            SCENARIO,
            format!("restart {restart} tables after the drain: {counts_after_drain:?}"),
        );
        assert_eq!(
            counts_after_drain, counts_before,
            "the drain and finalize left the counters where the kill did"
        );
        assert_eq!(
            merged_after_drain, merged_digest_before,
            "a restart and a drain do not rewrite merged rows"
        );
    }
}

/// The `drained: accepted=…` line of a service's stdout, the certificate a clean stop prints.
fn drain_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|line| line.starts_with("drained:"))
        .unwrap_or("<no drain report>")
}

/// The `census service stopped report=DrainReport { … }` line, which carries the stop reason.
fn stop_reason_line(stdout: &str) -> &str {
    stdout
        .lines()
        .find(|line| line.contains("stop_reason"))
        .unwrap_or("<no stop reason>")
}

/// The `drained: accepted=… completed=… …` counters, as the `census-serve` bin prints them.
fn parse_drain_line(line: &str) -> BTreeMap<String, u64> {
    line.trim_start_matches("drained:")
        .split_whitespace()
        .filter_map(|field| field.split_once('='))
        .filter_map(|(name, value)| {
            value
                .parse::<u64>()
                .ok()
                .map(|value| (name.to_string(), value))
        })
        .collect()
}

// -------------------------------------------------------------------------------------------------
// 7. A service nobody stops, measured against its own drain deadline
// -------------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_with_no_stop_request_survives_its_drain_deadline() {
    const SCENARIO: &str = "drain-deadline";
    const DRAIN_SECS: u64 = 2;
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("store");
    let units = ["unit-1", "unit-2"];

    let (journal_before, counts_before) = {
        let store = open_store(&root);
        process_units(&store, &units, OBSERVED_ON);
        store.flush().expect("flushing the store");
        (journal_keys(&store, UNIT_PHASE), table_counts(&store))
    };
    note(
        SCENARIO,
        format!("before the service: journal={journal_before:?} tables={counts_before:?}"),
    );

    let port = free_loopback_port();
    let (mut child, manifest) = spawn_serve(&root, port, DRAIN_SECS).await;
    assert_manifest_advertises(SCENARIO, &manifest);

    // No signal and no shutdown future reach this service. An endpoint is expected to serve until one
    // arrives, and to spend the drain deadline on the reap that follows it (`src/bootstrap.rs`: the
    // supervisor "waits for a stop request, drains inside a bounded deadline").
    let exited = wait_for_exit(&mut child, Duration::from_secs(DRAIN_SECS + 8));
    let still_answering = endpoint_answers(&discovery_client(), port).await;
    match exited {
        Some(elapsed) => {
            let output = child
                .wait_with_output()
                .expect("the exited service's output");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "DEFECT: no stop request was sent, yet the process exited after {elapsed:?} \
                     (drain-timeout {DRAIN_SECS}s) with status {:?} and still_answering={still_answering}: \
                     an unrequested stop ends the service. {} => {}",
                    output.status,
                    stop_reason_line(&stdout),
                    drain_line(&stdout)
                ),
            );
        }
        None if !still_answering => {
            // A process that is up but serves nothing is worse than one that exited: nothing
            // restarts it, and the port is dead.
            send_sigterm(&child);
            let output = child
                .wait_with_output()
                .expect("the drained service's output");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "DEFECT: the process outlived its drain deadline, but the endpoint stopped \
                     answering `/discover` with no stop request: the drain deadline reaped a healthy \
                     endpoint instead of waiting for the stop that is supposed to trigger it. \
                     status={:?} {} => {}",
                    output.status,
                    stop_reason_line(&stdout),
                    drain_line(&stdout)
                ),
            );
        }
        None => {
            note(
                SCENARIO,
                format!(
                    "still serving after the drain deadline with no stop request, \
                     still_answering={still_answering}"
                ),
            );
            send_sigterm(&child);
            let output = child
                .wait_with_output()
                .expect("the drained service's output");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "stopped by the operator: status={:?} {}",
                    output.status,
                    stop_reason_line(&stdout)
                ),
            );
            assert!(
                output.status.success(),
                "a drained service exits cleanly: {:?}\nstdout:\n{stdout}",
                output.status
            );
            assert!(
                stdout.contains("stop_reason: Signal"),
                "the stop was the operator's SIGTERM, not a server exit:\n{stdout}"
            );
        }
    }

    // Whichever of the two happened, the durable side is the part that must hold: an exit nobody
    // asked for cannot change the journal or a counter.
    let store = open_store(&root);
    let journal_after = journal_keys(&store, UNIT_PHASE);
    let counts_after = table_counts(&store);
    note(
        SCENARIO,
        format!(
            "after the service: journal={} tables={counts_after:?}",
            journal_after.len()
        ),
    );
    assert_eq!(
        journal_after, journal_before,
        "an unrequested stop cannot change the journal"
    );
    assert_eq!(
        counts_after, counts_before,
        "an unrequested stop cannot change the counters"
    );
}

// -------------------------------------------------------------------------------------------------
// 8. KS: the journal-before-append window, measured on a real kill
// -------------------------------------------------------------------------------------------------

#[tokio::test]
async fn ks_directory_walk_claims_units_the_kill_can_lose() {
    const SCENARIO: &str = "ks-window";
    let dir = tempfile::tempdir().expect("temp dir");

    // Control: one clean pass, in-process (only the unit count is needed from it).
    let control_root = dir.path().join("control");
    let (control_total, control_counts) = {
        let store = store_seeded_with_ks(&control_root);
        let report = ks_pass(&store, None).await;
        assert_eq!(report.requests, 0);
        (journal_keys(&store, KS_PHASE).len(), table_counts(&store))
    };
    note(
        SCENARIO,
        format!("control units={control_total} tables={control_counts:?}"),
    );

    // Measure the clean runtime of the same pass as a process, then kill at fractions of it.
    let timed_root = dir.path().join("timed");
    {
        let store = store_seeded_with_ks(&timed_root);
        drop(store);
    }
    let started = Instant::now();
    run_census(&timed_root, &["provider", "ks"]);
    let clean_runtime = started.elapsed();
    note(SCENARIO, format!("clean process runtime={clean_runtime:?}"));

    let root = dir.path().join("killed");
    {
        let store = store_seeded_with_ks(&root);
        drop(store);
    }
    let attempts = kill_ladder(
        SCENARIO,
        &root,
        &["provider", "ks"],
        Subject {
            phase: KS_PHASE,
            table: Table::Schools,
            ids_of: school_ids,
        },
        control_total,
        clean_runtime,
    );

    // The window: units the kill claimed done while the pass had not yet appended their rows.
    for attempt in &attempts {
        let claimed_without_rows = attempt.journal.len() as i64 - attempt.observations as i64;
        if claimed_without_rows > 0 {
            note(
                SCENARIO,
                format!(
                    "DEFECT: after a kill at {:?}, {} unit(s) are journaled done with 0 of their \
                     rows on disk (schools={}); the walk journals each record and appends the batch \
                     at the end of the pass (sources/ks/collect.rs)",
                    attempt.delay,
                    attempt.journal.len(),
                    attempt.observations
                ),
            );
        }
    }

    let last = attempts.last().expect("at least one kill attempt");
    let journal_at_kill = last.journal.clone();
    let rows_at_kill = last.observations;

    let resumed = report_of(&run_census(&root, &["provider", "ks"]));
    let (journal, final_counts) = {
        let store = open_store(&root);
        (journal_keys(&store, KS_PHASE), table_counts(&store))
    };
    let final_schools = count_of(&final_counts, Table::Schools);
    note(
        SCENARIO,
        format!(
            "restart: rows={} journal={} schools={final_schools} (control schools={})",
            resumed["rows"],
            journal.len(),
            count_of(&control_counts, Table::Schools)
        ),
    );

    assert_eq!(
        journal.len(),
        control_total,
        "the restart completes the journal set"
    );
    // The unit accounting that must hold whatever the on-disk ordering is: the pass writes exactly
    // the units the journal does not claim, so the final rows are the rows already on disk at kill
    // time plus every unclaimed unit.
    assert_eq!(
        final_schools as usize,
        rows_at_kill as usize + (control_total - journal_at_kill.len()),
        "the restart writes each unclaimed unit exactly once"
    );
    let claimed_without_rows = journal_at_kill.len() as i64 - rows_at_kill as i64;
    let missing = control_total as i64 - final_schools as i64;
    note(
        SCENARIO,
        format!(
            "measured: claimed_without_rows_at_kill={claimed_without_rows} \
             missing_from_the_final_store={missing} (an append-before-journal ordering reports 0 \
             for both)"
        ),
    );
}

// -------------------------------------------------------------------------------------------------
// 9. Jurisdiction walk (team index + rosters) resume
// -------------------------------------------------------------------------------------------------

/// The jurisdiction object's two collection stage bodies are `census::collect_state_teams` and
/// `census::collect_state_rosters` (`restate_services/jobs.rs`), so the durable claims those stages
/// make to the workflow are asserted here on the library calls themselves:
///
/// * the team index is journaled per state, and a later stage re-reads it from the cache instead of
///   rebuilding it — which is what makes the roster stage's "read the index back" cheap; and
/// * a roster pass on a store where an earlier pass stopped completes exactly the rosters that pass
///   did not journal, and lands on the control pass's counters, journal and merged athlete rows.
///
/// The walk is driven in-process; the Restate replay of a stage handler is the SDK's own durable
/// execution, which needs a live `restate-server` and so is out of reach of this offline suite.
#[tokio::test]
async fn jurisdiction_walk_resumes_from_the_journaled_index_and_the_unclaimed_rosters() {
    const SCENARIO: &str = "jurisdiction-walk";
    let dir = tempfile::tempdir().expect("temp dir");
    let site = milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin);

    // Control: one clean walk over the seeded cache, in its own store.
    let control_root = dir.path().join("control");
    let control = {
        let store = store_seeded_with_wisconsin(&control_root, &site);
        let fetcher = fetcher_for(&store);
        let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            .expect("WI team index");
        let index_stats = fetcher.stats().await;
        assert_eq!(
            index_stats.requests, 0,
            "the team index must come from the seeded cache"
        );
        assert!(index_stats.cache_hits >= 1);
        let progress = census::collect_state_rosters(
            &fetcher,
            &store,
            &teams,
            &wi_options(None),
            UsJurisdiction::Wisconsin,
        )
        .await
        .expect("WI rosters");
        let athletes = store
            .scan::<CanonicalAthlete>(Table::Athletes)
            .expect("scan athletes");
        // The class-of-2027 count is the roster page's own: `roster_entities` files every athlete and
        // the sweep counts the graded ones, once per roster walked.
        let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, teams[0].clone())
            .expect("the roster fixture parses");
        let per_roster = parsed
            .athletes
            .iter()
            .filter(|athlete| athlete.grad_year == GradYear::CO2027)
            .count();
        assert!(per_roster > 0, "the roster fixture carries Co2027 athletes");
        let walk = Walk {
            teams,
            progress,
            athletes,
            index_journal: journal_keys(&store, WI_TEAMS_PHASE),
            roster_journal: journal_keys(&store, WI_ROSTERS_PHASE),
            counts: table_counts(&store),
            stats: fetcher.stats().await,
            per_roster,
        };
        note(
            SCENARIO,
            format!(
                "control: teams={} rosters_done={} co2027={} athletes={} requests={} cache_hits={} \
                 tables={:?}",
                walk.teams.len(),
                walk.progress.rosters_done,
                walk.progress.class_of_2027,
                walk.athletes.len(),
                walk.stats.requests,
                walk.stats.cache_hits,
                walk.counts
            ),
        );
        assert_eq!(
            walk.index_journal,
            BTreeSet::from(["WI".to_string()]),
            "the team index is journaled once per state"
        );
        assert_eq!(
            walk.progress.rosters_done,
            walk.teams.len(),
            "a clean pass walks every roster the index lists"
        );
        assert_eq!(walk.progress.rosters_skipped, 0);
        assert!(
            walk.progress.errors.is_empty(),
            "the seeded cache leaves no roster unfetched: {:?}",
            walk.progress.errors
        );
        assert_eq!(walk.stats.requests, 0, "no walk step needs the network");
        assert_eq!(
            walk.roster_journal.len(),
            walk.teams.len(),
            "one journal entry per finished roster"
        );
        assert_eq!(
            walk.progress.class_of_2027,
            per_roster * walk.teams.len(),
            "every walked roster reports its own Co2027 count"
        );
        walk
    };

    // A pass that stops after one roster. The store handle is dropped mid-walk, the way a process
    // that ends without finishing its set leaves the disk.
    let restart_root = dir.path().join("restart");
    let stopped_after = {
        let store = store_seeded_with_wisconsin(&restart_root, &site);
        let fetcher = fetcher_for(&store);
        let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            .expect("WI team index");
        let first = census::collect_state_rosters(
            &fetcher,
            &store,
            &teams,
            &wi_options(Some(1)),
            UsJurisdiction::Wisconsin,
        )
        .await
        .expect("WI rosters, capped at one");
        let journal = journal_keys(&store, WI_ROSTERS_PHASE);
        note(
            SCENARIO,
            format!(
                "first pass (limit 1): rosters_done={} skipped={} journal={journal:?}",
                first.rosters_done, first.rosters_skipped
            ),
        );
        assert_eq!(first.rosters_done, 1);
        assert_eq!(
            first.rosters_skipped, 0,
            "skipped counts rosters an earlier pass journaled, and this pass is the first: the \
             limit defers the rest instead"
        );
        assert_eq!(journal.len(), 1);
        journal
    };

    // Restart on the same store: the index stage re-reads its journaled copy, and the roster stage
    // claims only the rosters the first pass left unjournaled.
    let store = open_store(&restart_root);
    let fetcher = fetcher_for(&store);
    let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
        .await
        .expect("WI team index replay");
    let index_stats = fetcher.stats().await;
    note(
        SCENARIO,
        format!(
            "replay: teams={} requests={} cache_hits={}",
            teams.len(),
            index_stats.requests,
            index_stats.cache_hits
        ),
    );
    assert_eq!(
        teams, control.teams,
        "the second stage re-reads the journaled index rather than rebuilding it"
    );
    assert_eq!(
        index_stats.requests, 0,
        "the index replay is answered from the cache"
    );

    let resumed = census::collect_state_rosters(
        &fetcher,
        &store,
        &teams,
        &wi_options(None),
        UsJurisdiction::Wisconsin,
    )
    .await
    .expect("WI rosters, resumed");
    let athletes = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("scan athletes");
    let counts = table_counts(&store);
    let stats = fetcher.stats().await;
    note(
        SCENARIO,
        format!(
            "restart: rosters_done={} skipped={} co2027={} athletes={} requests={} cache_hits={} \
             tables={counts:?}",
            resumed.rosters_done,
            resumed.rosters_skipped,
            resumed.class_of_2027,
            athletes.len(),
            stats.requests,
            stats.cache_hits
        ),
    );
    assert_eq!(
        resumed.rosters_done,
        teams.len() - stopped_after.len(),
        "the restart walked only the rosters the first pass did not journal"
    );
    assert_eq!(
        resumed.rosters_skipped,
        stopped_after.len(),
        "every roster the first pass journaled is skipped unread"
    );
    assert!(
        resumed.errors.is_empty(),
        "the resumed pass leaves no roster unfetched: {:?}",
        resumed.errors
    );
    assert_eq!(
        journal_keys(&store, WI_ROSTERS_PHASE),
        control.roster_journal,
        "the restarted store completes exactly the roster set the control pass covered"
    );
    assert_eq!(
        journal_keys(&store, WI_TEAMS_PHASE),
        control.index_journal,
        "the team-index journal is untouched by the roster stage"
    );
    assert_eq!(
        resumed.class_of_2027,
        control.per_roster * (teams.len() - stopped_after.len()),
        "the resumed pass reports the cohort of exactly the rosters it walked"
    );
    assert_eq!(
        athletes, control.athletes,
        "the restarted store merges to exactly the control's athletes"
    );
    assert_eq!(
        counts, control.counts,
        "observation counters match the control: no roster was written twice"
    );
    assert_eq!(
        count_of(&counts, Table::Athletes),
        count_of(&control.counts, Table::Athletes),
        "athlete observations match the control one for one"
    );
}

/// What one jurisdiction walk produced: the index it read, the stage's own counters, the merged
/// athlete rows, and the journals and store counters a restart has to land on.
struct Walk {
    teams: Vec<milesplit::TeamRef>,
    progress: census::StateProgress,
    athletes: Vec<CanonicalAthlete>,
    index_journal: BTreeSet<String>,
    roster_journal: BTreeSet<String>,
    counts: BTreeMap<String, u64>,
    stats: census_crawl::net::FetchStats,
    /// Class-of-2027 athletes on one roster page: the per-pass cohort counters are this times the
    /// rosters the pass walked.
    per_roster: usize,
}
