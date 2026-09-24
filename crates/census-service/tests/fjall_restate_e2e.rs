//! End-to-end checks across the storage substrate and the service shell:
//!
//! 1. [`store_round_trip_merges_observations_and_reports_stats`] — append/merge/scan/stats through
//!    the public `Store` API only.
//! 2. [`legacy_jsonl_journals_are_imported_once`] — the one-time import of pre-Fjall JSONL logs, and
//!    the proof that reopening does not import them twice.
//! 3. [`report_bests_and_workbook_chain_over_synthetic_entities`] — consolidate → census (both
//!    scopes) → best marks → workbook over a synthetic corpus, with counts asserted at every step.
//! 4. [`restate_endpoint_advertises_services_and_drains_on_request`] — the supervisor's endpoint
//!    answers `/discover` with the nine services the workflow is built from and returns a drain
//!    report on request.
//! 5. [`restate_endpoint_advertises_the_lane_when_it_serves_one`] — the same manifest with the
//!    headed lane configured: a deployment that serves the profile advertises `BrowserSession`, and
//!    still drains on request.
//!
//! `/discover` is answered by the SDK's own endpoint (see `restate-sdk`'s `endpoint::mod`, which
//! routes any path whose last segment is `discover`), so discovery is driven directly here: no
//! external Restate server is needed, and no network traffic leaves the machine.

use athleticnet_browser::BrowserSettings;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Id, Mark, ObservedGrade, SchoolId, SchoolYear, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_report::report::{self, Census, Scope};
use census_report::{bests, workbook};
use census_service::bootstrap::{serve_until, DrainReport, ServeOptions, StopReason};
use census_service::census;
use census_store::{Store, StoreStats, Table};
use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::task::JoinSet;
use url::Url;

/// A core adapter id: never one of `report::NON_CORE_SOURCE_IDS`, so `Scope::Core` keeps every
/// synthetic row.
const SOURCE_ID: &str = "mshsl_results";
const MEET_DATE: &str = "2026-05-02";
/// Manifest type the way `restate-server` asks for it; the SDK supports v2..=v4.
const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";
const DISCOVERY_ATTEMPTS: usize = 50;
const DISCOVERY_RETRY_DELAY: Duration = Duration::from_millis(100);
/// Wire names come from Restate struct names, so they are PascalCase (see `restate_services` docs):
/// the operator's read surface, the four heavy jobs it shares, the ingest object, and the sweeps and
/// workflows that drive them.
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

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

/// A synthetic corpus: `schools` schools (all WI), `athletes_per_school` athletes each, one team and
/// one meet per school, one event per athlete, and two performances per athlete.
struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    /// Event ids minted during generation; the merge must reduce `events` to exactly this count.
    distinct_events: HashSet<String>,
}

impl Corpus {
    fn append(&self, store: &Store) {
        store.append_many(Table::Schools, &self.schools).unwrap();
        store.append_many(Table::Teams, &self.teams).unwrap();
        store.append_many(Table::Athletes, &self.athletes).unwrap();
        store.append_many(Table::Meets, &self.meets).unwrap();
        store.append_many(Table::Events, &self.events).unwrap();
        store
            .append_many(Table::Performances, &self.performances)
            .unwrap();
    }
}

fn synthetic_corpus(school_count: usize, athletes_per_school: usize) -> Corpus {
    let mut corpus = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
        distinct_events: HashSet::new(),
    };
    for index in 0..school_count {
        add_school(&mut corpus, index, athletes_per_school);
    }
    corpus
}

fn add_school(corpus: &mut Corpus, index: usize, athletes_per_school: usize) {
    let name = format!("E2E School {index}");
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name.clone(),
        normalize_name(&name),
    );
    school.evidence.push(evidence());
    let team = CanonicalTeam {
        id: Id::mint("team", &[school_id.as_str(), "outdoor", "2026"]),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: SchoolYear::new(2025).expect("2025 is a season"),
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    let team_id = team.id.clone();
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        format!("E2E Invite {index}"),
        MEET_DATE,
        CompetitionLevel::Invitational,
    );
    meet.evidence.push(evidence());
    let meet_id = meet.id.clone();
    corpus.schools.push(school);
    corpus.teams.push(team);
    corpus.meets.push(meet);
    for slot in 0..athletes_per_school {
        add_athlete(corpus, index, slot, &school_id, &team_id, &meet_id);
    }
}

fn add_athlete(
    corpus: &mut Corpus,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &census_domain::model::TeamId,
    meet_id: &census_domain::model::MeetId,
) {
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("E2E Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).unwrap(),
        school_year: SchoolYear::new(2025).expect("2025 is a season"),
        source: SourceRef::id(SOURCE_ID),
    });
    athlete.evidence.push(evidence());

    let kind = match slot % 3 {
        0 => EventKind::Track800m,
        1 => EventKind::Track1600m,
        _ => EventKind::Track3200m,
    };
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..2 {
        let source_key = format!("e2e-{index}-{slot}-{attempt}");
        let id = CanonicalPerformance::mint(&athlete.id, meet_id, &kind, MEET_DATE, &source_key);
        let seconds = 130.0 + f64::from(u32::try_from(index + slot).unwrap()) / 10.0;
        corpus.performances.push(CanonicalPerformance {
            id,
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: event.id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(CentiSeconds::from_seconds_f64(seconds)),
            wind_mps: None,
            place: Some(u16::try_from(attempt + 1).unwrap()),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(Grade::new(11).unwrap()),
            evidence: vec![evidence()],
            source_key,
            source_athlete: None,
            retained_conflicts: Vec::new(),
        });
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
}

fn count_of(counts: &[(String, usize)], table: &str) -> usize {
    counts
        .iter()
        .find(|(name, _)| name == table)
        .map(|(_, count)| *count)
        .unwrap_or_else(|| panic!("consolidate reported no count for {table}"))
}

#[test]
fn store_round_trip_merges_observations_and_reports_stats() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let (mut first, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Round Trip High School",
        "round trip",
    );
    first
        .evidence
        .push(Evidence::parsed(SourceRef::id(SOURCE_ID), "2026-05-01"));
    let mut duplicate = first.clone();
    duplicate.city = Some("Madison".to_string());
    duplicate
        .evidence
        .push(Evidence::parsed(SourceRef::id(SOURCE_ID), "2026-05-02"));

    store.append(Table::Schools, &first).unwrap();
    store.append(Table::Schools, &duplicate).unwrap();
    let athlete = CanonicalAthlete::new(&school_id, "Ada Runner", GradYear::CO2027, Gender::Girls);
    store.append_many(Table::Athletes, &[athlete]).unwrap();
    store.flush().unwrap();

    let schools = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(
        schools.len(),
        1,
        "two observations of one id merge into one school"
    );
    assert_eq!(schools[0].id.as_str(), school_id.as_str());
    assert_eq!(
        schools[0].city.as_deref(),
        Some("Madison"),
        "merge fills a field the first observation left empty"
    );
    assert_eq!(
        schools[0].evidence.len(),
        2,
        "merge keeps the evidence of both observations"
    );

    let athletes = store.scan::<CanonicalAthlete>(Table::Athletes).unwrap();
    assert_eq!(athletes.len(), 1);
    assert_eq!(athletes[0].canonical_name, "Ada Runner");

    // Stats count observations, not merged entities: two school observations and one athlete.
    assert_observation_counts(&store.stats().unwrap());
}

/// `stats` counts observations rather than merged entities, and reports every table in the store.
fn assert_observation_counts(stats: &StoreStats) {
    let count = |table: &str| {
        stats
            .tables
            .iter()
            .find(|(name, _)| name == table)
            .map(|(_, count)| *count)
            .unwrap_or(0)
    };
    assert_eq!(count("schools"), 2, "two school observations");
    assert_eq!(count("athletes"), 1);
    assert_eq!(
        count("performances"),
        0,
        "a table with no writes counts zero"
    );
    assert_eq!(
        stats.tables.len(),
        Table::ALL.len(),
        "every table is reported"
    );
    assert_eq!(stats.observations, 3);
}

#[test]
fn legacy_jsonl_journals_are_imported_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Legacy High School", "legacy");
    school
        .evidence
        .push(Evidence::parsed(SourceRef::id(SOURCE_ID), "2026-04-01"));
    let mut later = school.clone();
    later
        .evidence
        .push(Evidence::parsed(SourceRef::id(SOURCE_ID), "2026-04-02"));
    let log = format!(
        "{}\n{}\n",
        serde_json::to_string(&school).unwrap(),
        serde_json::to_string(&later).unwrap()
    );
    std::fs::write(root.join("entities/schools.jsonl"), log).unwrap();
    std::fs::write(
        root.join("journal/mshsl_schools.jsonl"),
        "{\"key\":\"wi:1\",\"at\":\"2026-04-01\",\"payload\":{\"schools\":1}}\n",
    )
    .unwrap();

    {
        let store = Store::open(root).unwrap();
        // Opening a store is a read, so the one-time import is the caller's decision: this test is
        // the caller, exactly as the offline run and the `import-legacy` verb are.
        store.import_legacy().unwrap();
        let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        assert_eq!(
            rows.len(),
            1,
            "the legacy log holds one school under two observations"
        );
        assert_eq!(
            rows[0].evidence.len(),
            2,
            "the import keeps every observation"
        );
        assert!(store
            .journal_keys("mshsl_schools")
            .unwrap()
            .contains("wi:1"));
        assert_eq!(store.journal_payloads("mshsl_schools").unwrap().len(), 1);
    }

    // Reopen only after the first store is dropped: Fjall holds a file lock on its directory, so a
    // second live open of the same path is rejected by design. The import marker must now make the
    // importer a no-op instead of duplicating every observation.
    let store = Store::open(root).unwrap();
    // The marker the first import wrote makes this call a no-op: the second open must not duplicate
    // a single observation.
    store.import_legacy().unwrap();
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(
        rows.len(),
        1,
        "reopening must not import the legacy log a second time"
    );
    assert_eq!(rows[0].evidence.len(), 2);
    assert_eq!(store.journal_keys("mshsl_schools").unwrap().len(), 1);
}

#[test]
fn report_bests_and_workbook_chain_over_synthetic_entities() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let corpus = synthetic_corpus(3, 2);
    corpus.append(&store);

    let counts = census::consolidate(&store).unwrap();
    assert_eq!(count_of(&counts, "schools"), corpus.schools.len());
    assert_eq!(count_of(&counts, "teams"), corpus.teams.len());
    assert_eq!(count_of(&counts, "athletes"), corpus.athletes.len());
    assert_eq!(count_of(&counts, "meets"), corpus.meets.len());
    assert_eq!(count_of(&counts, "events"), corpus.distinct_events.len());
    assert_eq!(count_of(&counts, "performances"), corpus.performances.len());
    assert_eq!(count_of(&counts, "coaches"), 0);

    let core = report::build_census(&store, Scope::Core).unwrap();
    let all_sources = report::build_census(&store, Scope::AllSources).unwrap();
    assert_census_counts(&core, &all_sources, &corpus);

    let bests = bests::build(
        &store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    assert_eq!(
        bests.len(),
        corpus.athletes.len(),
        "one best mark per athlete"
    );
    assert!(
        bests.iter().all(|row| row.marks_in_event == 2),
        "each athlete rests on both performances"
    );

    let path = workbook::build(
        &store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(dir.path().join("e2e-census.xlsx")),
            limit: None,
            scope: Scope::Core,
        },
    )
    .unwrap();
    assert_complete_xlsx(&path);
}

/// Both scopes must count the synthetic corpus exactly, and every synthetic school sits in WI.
fn assert_census_counts(core: &Census, all_sources: &Census, corpus: &Corpus) {
    assert_eq!(core.scope, "core");
    assert_eq!(all_sources.scope, "all_sources");
    assert_eq!(core.totals.athletes, corpus.athletes.len());
    assert_eq!(core.totals.class_of_2027, corpus.athletes.len());
    assert_eq!(all_sources.totals.athletes, corpus.athletes.len());
    assert_eq!(
        core.by_state
            .get(&census_domain::JurisdictionBucket::from(
                census_domain::UsJurisdiction::Wisconsin,
            ))
            .map(|state| state.schools),
        Some(corpus.schools.len()),
        "every synthetic school is in WI"
    );
}

/// An xlsx is a zip: a local file header opens it and an end-of-central-directory record closes it,
/// which is exactly what a truncated or empty workbook would be missing.
fn assert_complete_xlsx(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.starts_with(b"PK\x03\x04"),
        "{} is not an xlsx (zip) container",
        path.display()
    );
    let eocd = bytes
        .len()
        .checked_sub(22)
        .expect("a complete xlsx carries a zip end-of-central-directory record");
    assert_eq!(
        &bytes[eocd..eocd + 4],
        b"PK\x05\x06".as_slice(),
        "workbook {} is a truncated zip container",
        path.display()
    );
}

/// Bind an ephemeral port, read it back, and drop the listener so `serve_until` can bind it. The
/// handoff window is the usual ephemeral-port TOCTOU: losing the race shows up as `serve_until`
/// returning a bind error, never as a silent pass.
fn free_local_address() -> SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("binding an ephemeral port");
    let address = listener.local_addr().expect("reading the bound address");
    drop(listener);
    address
}

/// Poll `/discover` until the endpoint answers, then return the advertised service names. The retry
/// loop is bounded by `DISCOVERY_ATTEMPTS`.
async fn discover_service_names(client: &reqwest::Client, url: &str) -> Vec<String> {
    let mut answered = None;
    let mut last_error = None;
    for _ in 0..DISCOVERY_ATTEMPTS {
        match client
            .get(url)
            .header("accept", DISCOVERY_ACCEPT)
            .send()
            .await
        {
            Ok(response) => {
                answered = Some(response);
                break;
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(DISCOVERY_RETRY_DELAY).await;
            }
        }
    }
    let Some(response) = answered else {
        panic!("{url} never answered within {DISCOVERY_ATTEMPTS} attempts: {last_error:?}");
    };
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "discovery must answer 200 while the endpoint is up"
    );
    let manifest: serde_json::Value = response.json().await.unwrap();
    manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .expect("the discovery manifest carries a services array")
        .iter()
        .filter_map(|service| service.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

/// The client the endpoint's discovery surface speaks: HTTP/2 with prior knowledge, because the
/// SDK's endpoint serves h2 only (hyper's `http2::Builder`) and the client cannot upgrade into it.
fn discovery_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .http2_prior_knowledge()
        .build()
        .unwrap()
}

/// Request shutdown, then reap the supervisor and return its drain report. A supervisor that
/// outlives the deadline has its region aborted before the test fails.
async fn drain_after_shutdown(
    tasks: &mut JoinSet<anyhow::Result<DrainReport>>,
    shutdown: tokio::sync::oneshot::Sender<()>,
) -> DrainReport {
    shutdown
        .send(())
        .expect("the supervisor still awaits the shutdown request");
    let outcome = tokio::time::timeout(Duration::from_secs(30), tasks.join_next()).await;
    let joined = match outcome {
        Ok(joined) => joined,
        Err(_) => {
            tasks.abort_all();
            panic!("serve_until did not return within 30s of the shutdown request");
        }
    };
    joined
        .expect("the serve task vanished without a drain report")
        .expect("the serve task panicked")
        .expect("serve_until returned an error")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restate_endpoint_advertises_services_and_drains_on_request() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let data_dir = dir.path().join("data");
    let options = ServeOptions {
        listen,
        data_dir: data_dir.clone(),
        max_concurrent: 4,
        drain_timeout: Duration::from_secs(5),
        // No lane: this endpoint is proved without a browser, which is the deployment shape that
        // serves the nine store-backed services only.
        lane: None,
    };

    // The shutdown future is the supervisor's "request" stage. The sender stays alive until the
    // endpoint has answered, so the server cannot stop before the assertions below run.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
    tasks.spawn(async move {
        serve_until(options, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let url = format!("http://{listen}/discover");
    let client = discovery_client();
    let names = discover_service_names(&client, &url).await;
    for expected in EXPECTED_SERVICES {
        assert!(
            names.iter().any(|name| name == expected),
            "the discovery manifest {names:?} does not advertise {expected}"
        );
    }
    assert_eq!(
        names.len(),
        EXPECTED_SERVICES.len(),
        "the endpoint advertises exactly its services, nothing more: {names:?}"
    );

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
    assert!(
        report.accepted >= 1,
        "no accepted work in the drain report: {report:?}"
    );
    assert_eq!(
        report.panicked, 0,
        "a task panicked during drain: {report:?}"
    );

    // Finalization dropped the Fjall database, so the directory can be opened again.
    let reopened = Store::open(&data_dir).unwrap();
    assert_eq!(reopened.stats().unwrap().tables.len(), Table::ALL.len());
}

/// A deployment that serves a lane advertises it. The manifest is what a client is bound against, so
/// a `BrowserSession` that is bound and not advertised is a lane no census can address. No browser
/// is launched: the object is constructed here and launches one only when an operator calls `start`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restate_endpoint_advertises_the_lane_when_it_serves_one() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let options = ServeOptions {
        listen,
        data_dir: dir.path().join("data"),
        max_concurrent: 4,
        drain_timeout: Duration::from_secs(5),
        lane: Some(BrowserSettings {
            cdp_endpoint: None,
            executable: PathBuf::from("/nonexistent/chromium"),
            profile_dir: dir.path().join("profile"),
            source_origin: Url::parse("https://www.athletic.net/").unwrap(),
            tabs: 1,
            request_timeout: Duration::from_secs(30),
            challenge_wait: Duration::from_secs(5),
            headed: true,
        }),
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
    tasks.spawn(async move {
        serve_until(options, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let url = format!("http://{listen}/discover");
    let client = discovery_client();
    let names = discover_service_names(&client, &url).await;
    assert!(
        names.iter().any(|name| name == "BrowserSession"),
        "the discovery manifest {names:?} does not advertise the lane"
    );
    assert_eq!(
        names.len(),
        EXPECTED_SERVICES.len() + 1,
        "the endpoint advertises its services and the lane, nothing more: {names:?}"
    );

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
}

/// The drain deadline bounds the reap after a stop request, never the wait for one: an endpoint
/// that has been given no signal and no shutdown keeps serving past `drain_timeout`, and a stop
/// that does arrive still drains inside the deadline instead of timing out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unrequested_stop_does_not_end_the_endpoint_at_the_drain_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let listen = free_local_address();
    let options = ServeOptions {
        listen,
        data_dir: dir.path().join("data"),
        max_concurrent: 4,
        drain_timeout: Duration::from_millis(250),
        lane: None,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let mut tasks: JoinSet<anyhow::Result<DrainReport>> = JoinSet::new();
    tasks.spawn(async move {
        serve_until(options, async move {
            let _ = shutdown_rx.await;
        })
        .await
    });

    let url = format!("http://{listen}/discover");
    let client = discovery_client();
    let names = discover_service_names(&client, &url).await;
    assert!(
        names.iter().any(|name| name == EXPECTED_SERVICES[0]),
        "the discovery manifest {names:?} does not advertise {}",
        EXPECTED_SERVICES[0]
    );

    // Outlive three drain deadlines with the stop still unrequested: draining before the watch
    // resolves would have ended the supervisor here and reported `ServerExit`.
    let deadlines = 3;
    tokio::time::sleep(Duration::from_millis(250) * deadlines).await;
    let still_serving = discover_service_names(&client, &url).await;
    assert!(
        still_serving
            .iter()
            .any(|name| name == EXPECTED_SERVICES[0]),
        "the endpoint stopped answering after {} drain deadlines: {still_serving:?}",
        deadlines
    );
    assert!(
        tasks.try_join_next().is_none(),
        "the supervisor returned on its own, without a stop request"
    );

    let report = drain_after_shutdown(&mut tasks, shutdown_tx).await;
    assert_eq!(report.stop_reason, StopReason::Requested);
    assert_eq!(
        report.timed_out, 0,
        "a stop request must drain inside the deadline: {report:?}"
    );
}
