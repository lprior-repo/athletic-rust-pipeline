//! The one claim `fjall_restate_e2e.rs` cannot make: a run that is **killed mid-flight** resumes
//! from its journal, and no durable write happens twice.
//!
//! That file says it plainly: discovery is answered by the SDK's own endpoint, "no external Restate
//! server is needed, and no network traffic leaves the machine". It proves the endpoint advertises
//! five services and drains on request. It cannot prove resume, because nothing holds the journal
//! and there is no second process to resume *in*.
//!
//! This file adds exactly those two things:
//!
//! * a real `restate-server` process (the pinned 1.7.10 build, or `$RESTATE_SERVER_BIN`), with its
//!   base-dir on local disk and the schema's durability knobs set the way the census node sets them;
//! * a real `census-serve` process, registered with that server as its deployment.
//!
//! Then it kills the endpoint with SIGKILL in the middle of a `Consolidate` run — the corpus is
//! large enough that the table scan and snapshot write are still running when the signal lands —
//! starts the endpoint again on the same port and data-dir, resumes the paused invocation, and
//! asserts three things:
//!
//! 1. the invocation is not lost: the killed run resumes and finishes its merge;
//! 2. it does not start a second execution: re-submitting the same run identity is refused as an
//!    invocation that already exists;
//! 3. nothing was written twice: the store's observation count still equals the corpus appended, and
//!    a replayed append would have doubled it. (Observations, not merged entities: `stats` counts
//!    the rows the store received, which is precisely what a replayed durable write would duplicate.)
//!
//! The resume is an explicit step because the node does not perform one: a killed endpoint's
//! invocation fails, backs off, and is **paused** once the retry policy is spent, and the operator's
//! recovery is `PATCH /invocations/{id}/resume` for each paused row (`HANDOFF.md` §"Evidence and
//! remaining work"). Restarting the endpoint alone leaves the run parked, so a test that only
//! restarted it would be asserting a redelivery the node never makes.
//!
//! `Consolidate` is the run this proves against because it is the one heavy job that touches only
//! the store: a jurisdiction census walks the source sites, so a kill mid-census would make this
//! test depend on the network it is meant to be independent of.
//!
//! The test is skipped — loudly, and only — when no server binary can be found, because a test that
//! silently passes without a server would be worse than no test. Everything else is deterministic:
//! ports are taken from the kernel, all state lives under one `mkdtemp`-style directory, and both
//! child processes are killed on drop.

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Id, Mark, ObservedGrade, SchoolId, SchoolYear, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_store::{Store, Table};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The pinned server build, or anything the operator points at.
const SERVER_ENV: &str = "RESTATE_SERVER_BIN";
/// A core adapter id: never one of `report::NON_CORE_SOURCE_IDS`, so a core-scope census keeps every
/// synthetic row.
const SOURCE_ID: &str = "mshsl_results";
const MEET_DATE: &str = "2026-05-02";
const SEASON: SchoolYear = SchoolYear::new(2025).expect("2025 is a season");
const REVISION: Revision = Revision(1);

/// Big enough that the `Consolidate` merge is still working when the kill lands. 300 schools × 40
/// athletes = 12 000 athletes and 24 000 performances; the scan-and-write of the athlete and
/// performance snapshots alone is seconds of work.
const SCHOOLS: usize = 300;
const ATHLETES_PER_SCHOOL: usize = 40;

/// How long to let the invocation get into the store before killing the endpoint under it. Short:
/// the node accepts a submission in tens of milliseconds, and the merge is the long part, so a kill
/// this early lands inside the merge rather than after it.
const KILL_DELAY: Duration = Duration::from_millis(300);
/// Readiness and completion budgets. The merge is seconds; two minutes is the wedged-connection
/// budget, not the expected duration.
const READY_BUDGET: Duration = Duration::from_secs(60);
const RUN_BUDGET: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

/// The server binary: `$RESTATE_SERVER_BIN`, then the pinned build, then `PATH`. `None` skips the
/// test with a printed reason.
fn server_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SERVER_ENV) {
        return Some(PathBuf::from(path));
    }
    let pinned = std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local/share/athletic-rust-pipeline/restate/1.7.10/restate-server")
    });
    if let Some(path) = pinned {
        if path.is_file() {
            return Some(path);
        }
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("restate-server"))
            .find(|candidate| candidate.is_file())
    })
}

/// Bind an ephemeral port, read it back, and drop the listener so a child can bind it. The handoff
/// window is the usual ephemeral-port TOCTOU: losing the race shows up as a child that never
/// becomes ready, never as a silent pass.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
    let port = listener
        .local_addr()
        .expect("read the bound address")
        .port();
    drop(listener);
    port
}

/// A child process that is killed when the test ends, however it ends.
struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    /// SIGKILL: the point of the test is that no shutdown hook runs.
    fn kill_hard(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// SIGTERM, then wait for the drain. The store is a single-writer Fjall database: the test may
    /// only open it after the process that holds it has exited.
    fn stop_gracefully(&mut self) {
        let pid = self.child.id();
        // No `libc` in dev-dependencies: `kill` the shell command is the portable-enough form here,
        // and a failed kill just means the process already exited.
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100))
                }
                _ => break,
            }
        }
        self.kill_hard();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The Restate node the test runs against: its own base-dir, its own three ports, telemetry off.
struct Node {
    /// Held for its `Drop`: the server is killed when the node goes out of scope.
    _guard: ChildGuard,
    ingress: String,
    admin: String,
    /// Where the node's own output went; read it from a failure message.
    log_path: PathBuf,
}

impl Node {
    fn start(root: &Path) -> Node {
        let binary = server_binary().unwrap_or_else(|| {
            panic!(
                "no restate-server found: set {SERVER_ENV} or install one; \
                 this test must not silently pass without a server"
            )
        });
        let node_port = free_port();
        let ingress_port = free_port();
        let admin_port = free_port();
        let base = root.join("node");
        std::fs::create_dir_all(&base).expect("create the node base-dir");
        let config = root.join("restate.toml");
        std::fs::write(
            &config,
            format!(
                "roles = [\"http-ingress\", \"admin\", \"worker\", \"log-server\", \"metadata-server\"]\n\
                 node-name = \"kill-restart\"\n\
                 cluster-name = \"kill-restart\"\n\
                 auto-provision = true\n\
                 default-num-partitions = 1\n\
                 default-replication = 1\n\
                 base-dir = \"{base}\"\n\
                 listen-mode = \"tcp\"\n\
                 bind-ip = \"127.0.0.1\"\n\
                 bind-port = {node_port}\n\
                 advertised-address = \"http://127.0.0.1:{node_port}/\"\n\
                 shutdown-timeout = \"1m\"\n\
                 disable-telemetry = true\n\
                 experimental-enable-protocol-v7 = true\n\
                 experimental-enable-vqueues = true\n\
                 experimental-enable-scoped-virtual-objects = true\n\
                 \n\
                 [bifrost]\n\
                 default-provider = \"replicated\"\n\
                 \n\
                 [worker]\n\
                 durability-mode = \"replica-set-only\"\n\
                 \n\
                 [admin]\n\
                 bind-port = {admin_port}\n\
                 advertised-address = \"http://127.0.0.1:{admin_port}/\"\n\
                 \n\
                 [ingress]\n\
                 bind-port = {ingress_port}\n",
                base = base.display(),
            ),
        )
        .expect("write the node config");
        // Kept for the same reason the endpoint's log is: redelivery of the interrupted invocation
        // is the node's decision, and its log is where that decision is visible.
        let log_path = root.join("restate-server.log");
        let log = std::fs::File::create(&log_path).expect("create the node log");
        let child = Command::new(&binary)
            .arg("--no-logo")
            .arg("--config-file")
            .arg(&config)
            .stdout(Stdio::from(log.try_clone().expect("clone the node log")))
            .stderr(Stdio::from(log))
            .spawn()
            .expect("spawn restate-server");
        Node {
            _guard: ChildGuard { child },
            ingress: format!("http://127.0.0.1:{ingress_port}/"),
            admin: format!("http://127.0.0.1:{admin_port}/"),
            log_path,
        }
    }
}

/// The census endpoint: `census-serve`, the same binary the deployment runs.
struct Endpoint {
    guard: ChildGuard,
    listen: SocketAddr,
    /// Where the endpoint's own output went. A resumed invocation that stops making progress says
    /// why only in its log, so a test that discards stdout and stderr can report a symptom and
    /// nothing else.
    log_path: PathBuf,
}

impl Endpoint {
    fn start(data_dir: &Path, port: u16) -> Endpoint {
        let binary = env!("CARGO_BIN_EXE_census-serve");
        let listen = SocketAddr::from(([127, 0, 0, 1], port));
        let log_path = data_dir.join(format!("census-serve-{port}.log"));
        let log = std::fs::File::create(&log_path).expect("create the endpoint log");
        let child = Command::new(binary)
            .arg("--listen")
            .arg(listen.to_string())
            .arg("--data-dir")
            .arg(data_dir)
            .stdout(Stdio::from(
                log.try_clone().expect("clone the endpoint log"),
            ))
            .stderr(Stdio::from(log))
            .spawn()
            .expect("spawn census-serve");
        Endpoint {
            guard: ChildGuard { child },
            listen,
            log_path,
        }
    }

    /// The endpoint's output, for a failure message.
    fn log(&self) -> String {
        std::fs::read_to_string(&self.log_path).unwrap_or_default()
    }
}

/// Ask the node to (re)register the endpoint, retrying until it accepts: registration fails while
/// the endpoint is not yet listening, which makes it the readiness probe as well.
async fn register(
    client: &reqwest::Client,
    node: &Node,
    endpoint: &Endpoint,
) -> Result<(), String> {
    let uri = format!("http://{}/", endpoint.listen);
    let deadline = Instant::now() + READY_BUDGET;
    let mut last = String::from("no attempt made");
    while Instant::now() < deadline {
        match client
            .post(format!("{}deployments", node.admin))
            .json(&serde_json::json!({ "uri": uri }))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last = format!("status {}", response.status()),
            Err(error) => last = error.to_string(),
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    Err(format!("endpoint never registered: {last}"))
}

/// Wait for the node's admin API to answer at all.
async fn wait_for_node(client: &reqwest::Client, node: &Node) -> Result<(), String> {
    let deadline = Instant::now() + READY_BUDGET;
    let mut last = String::from("no attempt made");
    while Instant::now() < deadline {
        match client
            .get(format!("{}deployments", node.admin))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last = format!("status {}", response.status()),
            Err(error) => last = error.to_string(),
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    Err(format!("node admin API never came up: {last}"))
}

/// The invocation the kill left behind, read from the admin's `sys_invocation` table.
///
/// The node's retry policy spends its attempts against the closed socket and then **pauses** the
/// invocation — the same `on_max_attempts = "pause"` policy the live node runs, which `HANDOFF.md`
/// §"Evidence and remaining work" records as a paused run rather than a resumed one. The operator's
/// recovery enumerates those rows and resumes each, so the id comes from the same place here.
async fn paused_invocation(client: &reqwest::Client, node: &Node) -> Result<String, String> {
    let response = client
        .post(format!("{}query", node.admin))
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "query": "SELECT id, status FROM sys_invocation \
                      WHERE target_service_name = 'Consolidate' ORDER BY created_at DESC;"
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("the admin query answered {status}: {text}"));
    }
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{error}: {text}"))?;
    // Column names are fixed; the envelope around them is not (this build answers a `rows` object,
    // others a bare array), and there is exactly one `Consolidate` invocation on this node, so the
    // id is read by looking for the row rather than by pinning the wrapper.
    let id = find_key(&body, "id").and_then(|value| value.as_str().map(str::to_string));
    id.ok_or_else(|| format!("no invocation id in the admin's answer: {text}"))
}

/// The first value under `key` anywhere in `value`, depth first.
fn find_key<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|child| find_key(child, key))),
        serde_json::Value::Array(items) => items.iter().find_map(|child| find_key(child, key)),
        _ => None,
    }
}

/// Resume a paused invocation. `PATCH` is the verb the admin API takes; `POST` answers `405`.
async fn resume(client: &reqwest::Client, node: &Node, invocation: &str) -> Result<(), String> {
    let response = client
        .patch(format!("{}invocations/{invocation}/resume", node.admin))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let text = response.text().await.unwrap_or_default();
    Err(format!("the admin answered {status} to the resume: {text}"))
}

/// POST a handler and return the raw body, distinguishing transport failure from a handler reply.
///
/// The body is optional because a shared handler takes no input: Restate's ingress rejects a request
/// that carries one where the handler expects none, which surfaces as `400 input validation error`
/// and reads like a handler fault when it is not one.
async fn invoke(
    client: &reqwest::Client,
    node: &Node,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let request = client.post(format!("{}{}", node.ingress, path));
    let response = match body {
        Some(body) => request.json(&body).send().await,
        None => request.send().await,
    }
    .map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status.is_success() {
        serde_json::from_str(&text).map_err(|error| format!("{error}: {text}"))
    } else {
        Err(format!("{status}: {text}"))
    }
}

// ---------------------------------------------------------------------------------------------
// The synthetic corpus: the same shape the in-process test builds, sized for a kill window.
// ---------------------------------------------------------------------------------------------

struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
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

    /// Rows the store received. `stats` counts observations, so this is what a replayed append
    /// would double.
    fn appended_rows(&self) -> usize {
        self.schools.len()
            + self.teams.len()
            + self.athletes.len()
            + self.meets.len()
            + self.events.len()
            + self.performances.len()
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
    };
    for index in 0..school_count {
        add_school(&mut corpus, index, athletes_per_school);
    }
    corpus
}

fn add_school(corpus: &mut Corpus, index: usize, athletes_per_school: usize) {
    let name = format!("Kill Test School {index}");
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
        school_year: SEASON,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    let team_id = team.id.clone();
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        format!("Kill Test Invite {index}"),
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
        format!("Kill Test Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).unwrap(),
        school_year: SEASON,
        source: SourceRef::id(SOURCE_ID),
    });
    let athlete_id = athlete.id.clone();
    corpus.athletes.push(athlete);

    let kind = EventKind::Track100m;
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    let event_id = event.id.clone();
    corpus.events.push(event);

    for attempt in 0..2 {
        let source_key = format!("kill-{index}-{slot}-{attempt}");
        let id = CanonicalPerformance::mint(&athlete_id, meet_id, &kind, MEET_DATE, &source_key);
        corpus.performances.push(CanonicalPerformance {
            id,
            athlete: athlete_id.clone(),
            team: team_id.clone(),
            event: event_id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(CentiSeconds::from_seconds_f64(
                11.5 + f64::from(u32::try_from(attempt).unwrap()) / 10.0,
            )),
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
}

// ---------------------------------------------------------------------------------------------
// The test.
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write() {
    if server_binary().is_none() {
        panic!(
            "no restate-server found: set {SERVER_ENV} or install one. Refusing to pass without \
             the server this test exists to prove against."
        );
    }

    let root = std::env::temp_dir().join(format!("midwest-kill-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create the test root");
    let data_dir = root.join("census");
    std::fs::create_dir_all(&data_dir).expect("create the census data-dir");

    // 1. Build the corpus offline, through the same public store API the census uses. This is the
    //    work the killed run will be doing when the signal lands.
    let corpus = synthetic_corpus(SCHOOLS, ATHLETES_PER_SCHOOL);
    let expected_observations = corpus.appended_rows();
    {
        let store = Store::open(&data_dir).expect("open the store to seed the corpus");
        corpus.append(&store);
    }

    // 2. A real node, and a real endpoint registered with it.
    let node_guard = Node::start(&root);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("build the HTTP client");
    wait_for_node(&client, &node_guard)
        .await
        .expect("the node's admin API answers");

    let endpoint_port = free_port();
    let mut endpoint = Endpoint::start(&data_dir, endpoint_port);
    register(&client, &node_guard, &endpoint)
        .await
        .expect("the endpoint registers");
    // From here the endpoint guard is reused for restarts, so keep the port for the respawn.

    // The consolidate run's identity: one national run per season and revision, which is the run
    // whose merge must not be repeated. The key names the job instance; an empty table list means
    // every table.
    let key = WorkflowIdentity::national(
        SEASON,
        REVISION,
        &census_domain::UsJurisdiction::CENSUS_SCOPE,
    );
    let key = key.as_str().to_string();
    let path_run = format!("Consolidate/{key}/run");
    let request = serde_json::json!({ "tables": [] });

    // 3. Submit, let it get into the store, then kill the endpoint underneath it. The submission
    //    future dies with the connection; that is the point.
    let submission = {
        let client = client.clone();
        let ingress = node_guard.ingress.clone();
        let path = path_run.clone();
        let body = request.clone();
        tokio::spawn(async move {
            client
                .post(format!("{ingress}{path}"))
                .json(&body)
                .send()
                .await
                .map(|response| response.status().as_u16())
                .map_err(|error| error.to_string())
        })
    };
    tokio::time::sleep(KILL_DELAY).await;
    endpoint.guard.kill_hard();
    let killed = submission.await;
    // The client either saw the reset or (if the kill landed before the request left) a refusal;
    // both are reported, neither is an assertion, because the kill is the experiment.
    match &killed {
        Ok(Ok(status)) => eprintln!("note: submission answered {status} before the kill landed"),
        Ok(Err(error)) => eprintln!("note: submission failed as expected: {error}"),
        Err(error) => eprintln!("note: submission task panicked: {error}"),
    }

    // The merge's own output is the completion signal this test reads (step 6), and the count taken
    // here is what makes it a proof rather than a coincidence: taken immediately after the signal,
    // before anything could resume, it says how far the merge got before it was killed.
    //
    // The publish location is `<store>/out/<table>.jsonl` — the job's own destination, the one the
    // CLI consolidates to and every reader opens. `Store::table_path` is the pre-Fjall journal the
    // one-time import reads, not a snapshot output, so counting under `entities/` counted nothing
    // (0 of 15, every run) after the job moved here.
    let snapshot_dir = data_dir.join("out");
    // One `stat` per table, not per `.jsonl` in the directory: `out/` also holds the run's other
    // artifacts (bests, seal, sweep), and one of them landing must not read as a merged table.
    let snapshots_written = || {
        Table::ALL
            .into_iter()
            .filter(|table| {
                snapshot_dir
                    .join(format!("{}.jsonl", table.file()))
                    .is_file()
            })
            .count()
    };
    let at_kill = snapshots_written();
    assert!(
        at_kill < Table::ALL.len(),
        "the merge had already written every one of its {} snapshots before the kill landed, so \
         this run never had to resume",
        Table::ALL.len()
    );

    // 4. The run must still be known to the node: the journal is on disk, and the invocation was not
    //    completed. Re-registering the restarted endpoint is what lets the node redeliver it.
    let mut endpoint = Endpoint::start(&data_dir, endpoint_port);
    register(&client, &node_guard, &endpoint)
        .await
        .expect("the restarted endpoint registers");

    // 5. Re-submitting the same identity must not start a second execution. A workflow invocation
    //    is one-shot — the id is the key and the node keeps the invocation the kill left in flight —
    //    so the node refuses the repeat instead of forking the merge. That refusal is the proof.
    let attaching = reqwest::Client::builder()
        .build()
        .expect("build the attaching client");
    let repeat = tokio::time::timeout(
        RUN_BUDGET,
        invoke(&attaching, &node_guard, &path_run, Some(request.clone())),
    )
    .await;
    match repeat {
        Ok(Err(error)) => assert!(
            error.contains("409") && error.contains("already invoked"),
            "the repeat was not refused as an existing invocation: {error}"
        ),
        Ok(Ok(reply)) => panic!("the repeat started or answered a second execution: {reply}"),
        Err(_) => panic!("the repeat never answered"),
    }
    eprintln!("note: the repeat was refused as an existing invocation, so nothing forked");

    // 5b. The node does not redeliver on its own. Its retry policy spends the attempts a closed
    //     socket allows and then pauses the invocation, which is exactly the state the live node
    //     leaves a run in (`HANDOFF.md` §"Evidence and remaining work": recovery is
    //     `PATCH /invocations/{id}/resume` per paused row, never a fresh submission). Resuming it
    //     is the operator's recovery step, and without it this test would be asserting a
    //     redelivery the node never performs.
    let invocation = paused_invocation(&client, &node_guard)
        .await
        .expect("the admin names the paused invocation");
    resume(&client, &node_guard, &invocation)
        .await
        .expect("the paused invocation resumes");
    eprintln!("note: the paused invocation {invocation} was resumed");

    // 6. The resume proof. This deployment registers no read handler for a consolidate run — a
    //    workflow is answered by the invocation that owns it — so the merge's own output is the
    //    completion signal: the pass writes one JSONL snapshot per table, in order, and the run is
    //    done when the last one is on disk. The kill landed with only some of them written, so
    //    reaching all of them can only mean the redelivered invocation finished the merge.
    let deadline = Instant::now() + RUN_BUDGET;
    let mut written = at_kill;
    while Instant::now() < deadline && written < Table::ALL.len() {
        tokio::time::sleep(POLL_INTERVAL).await;
        written = snapshots_written();
    }
    assert!(
        written == Table::ALL.len(),
        "the run never finished its merge after the endpoint restarted: {written} of {} snapshots \
         landed (at the kill: {at_kill})\n\
         --- endpoint log ({}):\n{}\n--- node log ({}):\n{}",
        Table::ALL.len(),
        endpoint.log_path.display(),
        endpoint.log(),
        node_guard.log_path.display(),
        std::fs::read_to_string(&node_guard.log_path).unwrap_or_default(),
    );

    // 7. Nothing was written twice. Observations, not merged entities: a replayed durable write
    //    would append the whole corpus again and double this.
    endpoint.guard.stop_gracefully();
    let store = Store::open(&data_dir).expect("reopen the store after the endpoint drained");
    let stats = store.stats().expect("read store stats");
    assert_eq!(
        stats.observations, expected_observations as u64,
        "a resumed run replayed a durable write: expected {expected_observations} observations, \
         found {}",
        stats.observations
    );

    // And the merge landed: an invocation that resumed into a no-op would satisfy the count above
    // while merging nothing at all.
    let athletes_snapshot = data_dir.join("out").join("athletes.jsonl");
    let rows = std::fs::read_to_string(&athletes_snapshot)
        .expect("the consolidate snapshot for athletes")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert_eq!(
        rows,
        corpus.athletes.len(),
        "the athletes snapshot holds {rows} rows, not the corpus's {}",
        corpus.athletes.len()
    );

    drop(node_guard);
    let _ = std::fs::remove_dir_all(&root);
}
