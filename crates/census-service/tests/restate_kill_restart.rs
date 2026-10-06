#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Id, Mark, ObservedGrade, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_store::{Store, Table};
use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const SERVER_ENV: &str = "RESTATE_SERVER_BIN";
const SOURCE_ID: &str = "mshsl_results";
const MEET_DATE: &str = "2026-05-02";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn season() -> TestResult<SchoolYear> {
    SchoolYear::new(2025).ok_or_else(|| "invalid fixture season".into())
}
const REVISION: Revision = Revision(1);

const SCHOOLS: usize = 300;
const ATHLETES_PER_SCHOOL: usize = 40;

const KILL_DELAY: Duration = Duration::from_millis(300);
const READY_BUDGET: Duration = Duration::from_secs(60);
const RUN_BUDGET: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

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

static HANDED_OUT: LazyLock<Mutex<HashSet<u16>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn free_port() -> TestResult<u16> {
    for _ in 0..128 {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let first_time = match HANDED_OUT.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
        .insert(port);
        if first_time {
            return Ok(port);
        }
    }
    Err("no distinct ephemeral port after 128 attempts".into())
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn kill_hard(&mut self) -> TestResult {
        self.child.kill()?;
        let status = self.child.wait()?;
        check!(eq; status.signal(), Some(9), "the child must die from SIGKILL");
        Ok(())
    }

    fn stop_gracefully(&mut self) {
        let pid = self.child.id();
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
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Node {
    _guard: ChildGuard,
    ingress: String,
    admin: String,
    log_path: PathBuf,
}

impl Node {
    fn start(root: &Path) -> TestResult<Node> {
        let binary = server_binary().ok_or_else(|| format!(
            "no restate-server found: set {SERVER_ENV} or install one; test cannot run without a server"
        ))?;
        let node_port = free_port()?;
        let ingress_port = free_port()?;
        let admin_port = free_port()?;
        let base = root.join("node");
        std::fs::create_dir_all(&base)?;
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
        )?;
        let log_path = root.join("restate-server.log");
        let log = std::fs::File::create(&log_path)?;
        let child = Command::new(&binary)
            .arg("--no-logo")
            .arg("--config-file")
            .arg(&config)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?;
        Ok(Node {
            _guard: ChildGuard { child },
            ingress: format!("http://127.0.0.1:{ingress_port}/"),
            admin: format!("http://127.0.0.1:{admin_port}/"),
            log_path,
        })
    }

    fn restart(&mut self) -> TestResult {
        self._guard.kill_hard()?;
        let config_path = self
            .log_path
            .parent()
            .ok_or("node log carries no parent")?
            .join("restate.toml");
        let log_path = self.log_path.clone();
        let binary = server_binary().ok_or("restate-server unavailable for restart")?;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let child = Command::new(&binary)
            .arg("--no-logo")
            .arg("--config-file")
            .arg(&config_path)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?;
        self._guard = ChildGuard { child };
        Ok(())
    }
}

struct Endpoint {
    guard: ChildGuard,
    listen: SocketAddr,
    log_path: PathBuf,
}

impl Endpoint {
    fn start(data_dir: &Path, port: u16) -> TestResult<Endpoint> {
        let binary = env!("CARGO_BIN_EXE_census-serve");
        let listen = SocketAddr::from(([127, 0, 0, 1], port));
        let log_path = data_dir.join(format!("census-serve-{port}.log"));
        let log = std::fs::File::create(&log_path)?;
        let child = Command::new(binary)
            .arg("--listen")
            .arg(listen.to_string())
            .arg("--data-dir")
            .arg(data_dir)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?;
        Ok(Endpoint {
            guard: ChildGuard { child },
            listen,
            log_path,
        })
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log_path).map_or(Default::default(), core::convert::identity)
    }
}

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

async fn wait_for_node(client: &reqwest::Client, node: &Node) -> Result<(), String> {
    let deadline = Instant::now() + READY_BUDGET;
    let mut last = String::from("no attempt made");
    for _ in 0..240 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match client
            .post(format!("{}query", node.admin))
            .header("accept", "application/json")
            .json(&serde_json::json!({
                "query": "SELECT id FROM sys_invocation LIMIT 1"
            }))
            .timeout(remaining)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last = format!("status {}", response.status()),
            Err(error) => last = error.to_string(),
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
    }
    Err(format!("node invocation query never became ready: {last}"))
}

async fn paused_invocation(
    client: &reqwest::Client,
    node: &Node,
    deadline: Instant,
) -> Result<String, String> {
    let mut last = String::from("the admin query was never asked");
    for _ in 0..1_200 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let response = client
            .post(format!("{}query", node.admin))
            .header("accept", "application/json")
            .json(&serde_json::json!({
                "query": "SELECT id, status FROM sys_invocation \
                          WHERE target_service_name = 'Consolidate' AND status = 'paused' ORDER BY created_at DESC;"
            }))
            .timeout(remaining)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            last = format!("the admin query answered {status}: {text}");
            let remaining = deadline.saturating_duration_since(Instant::now());
            tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
            continue;
        }
        let body: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| format!("{error}: {text}"))?;
        let rows = body
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("no rows in the admin's answer: {text}"))?;
        if Instant::now() >= deadline {
            break;
        }
        if let Some(row) = rows.first() {
            let id = row
                .get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| format!("no invocation id in the admin's answer: {text}"))?;
            return Ok(id.to_string());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
    }
    Err(format!(
        "the killed endpoint's invocation never entered its paused state: {last}"
    ))
}

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
    let text = response
        .text()
        .await
        .map_or(Default::default(), core::convert::identity);
    Err(format!("the admin answered {status} to the resume: {text}"))
}

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
    let text = response
        .text()
        .await
        .map_or(Default::default(), core::convert::identity);
    if status.is_success() {
        serde_json::from_str(&text).map_err(|error| format!("{error}: {text}"))
    } else {
        Err(format!("{status}: {text}"))
    }
}

struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
}

impl Corpus {
    fn append(&self, store: &Store) -> TestResult {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }

    fn appended_rows(&self) -> usize {
        self.schools.len()
            + self.teams.len()
            + self.athletes.len()
            + self.meets.len()
            + self.events.len()
            + self.performances.len()
    }
}

fn synthetic_corpus(school_count: usize, athletes_per_school: usize) -> TestResult<Corpus> {
    let mut corpus = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
    };
    for index in 0..school_count {
        add_school(&mut corpus, index, athletes_per_school)?;
    }
    Ok(corpus)
}

fn add_school(corpus: &mut Corpus, index: usize, athletes_per_school: usize) -> TestResult {
    let name = format!("Kill Test School {index}");
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name.clone(),
        normalize_name(&name),
        None,
    );
    school.evidence.push(evidence());
    let team = CanonicalTeam {
        id: Id::mint("team", &[school_id.as_str(), "outdoor", "2026"]),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: season()?,
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
        add_athlete(corpus, index, slot, &school_id, &team_id, &meet_id)?;
    }
    Ok(())
}

fn add_athlete(
    corpus: &mut Corpus,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &census_domain::model::TeamId,
    meet_id: &census_domain::model::MeetId,
) -> TestResult {
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let source = SourceIdentity::new(
        SourceNamespace::Other("fixture".to_string()),
        format!("kill-test-athlete-{index}-{slot}"),
    );
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Kill Test Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
        source.clone(),
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).ok_or("invalid fixture grade")?,
        school_year: season()?,
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
            mark: Mark::TimeSeconds(
                CentiSeconds::try_from_seconds_f64(
                    11.5 + f64::from(u32::try_from(attempt)?) / 10.0,
                )
                .ok_or("invalid fixture time")?,
            ),
            wind_mps: None,
            place: Some(u16::try_from(attempt + 1)?),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(Grade::new(11).ok_or("invalid fixture grade")?),
            evidence: vec![evidence()],
            source_key,
            source_athlete: Some(source.clone()),
            retained_conflicts: Vec::new(),
        });
    }
    Ok(())
}

#[test]
fn a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async {
            server_binary().ok_or_else(|| {
                format!("no restate-server found: set {SERVER_ENV} or install one")
            })?;

            let root =
                std::env::temp_dir().join(format!("midwest-kill-restart-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root)?;
            let data_dir = root.join("census");
            std::fs::create_dir_all(&data_dir)?;

            let corpus = synthetic_corpus(SCHOOLS, ATHLETES_PER_SCHOOL)?;
            let expected_observations = corpus.appended_rows();
            {
                let store = Store::open(&data_dir)?;
                corpus.append(&store)?;
            }

            let node_guard = Node::start(&root)?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()?;
            wait_for_node(&client, &node_guard).await?;

            let endpoint_port = free_port()?;
            let mut endpoint = Endpoint::start(&data_dir, endpoint_port)?;
            register(&client, &node_guard, &endpoint).await?;

            let key = WorkflowIdentity::national(
                season()?,
                REVISION,
                &census_domain::UsJurisdiction::CENSUS_SCOPE,
            );
            let key = key.as_str().to_string();
            let path_run = format!("Consolidate/{key}/run");
            let request = serde_json::json!({ "tables": [] });

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
            endpoint.guard.kill_hard()?;
            submission.abort();
            let killed = submission.await;
            match &killed {
                Ok(Ok(status)) => {
                    eprintln!("note: submission answered {status} before the kill landed")
                }
                Ok(Err(error)) => eprintln!("note: submission failed as expected: {error}"),
                Err(error) if error.is_cancelled() => {
                    eprintln!("note: original ingress caller cancelled after the endpoint kill")
                }
                Err(error) => return Err(format!("submission task panicked: {error}").into()),
            }

            let snapshot_dir = data_dir.join("out");
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
            check!(at_kill < Table::ALL.len(),
    "the merge had already written every one of its {} snapshots before the kill landed, so \
     this run never had to resume",
    Table::ALL.len());

            let invocation =
                paused_invocation(&client, &node_guard, Instant::now() + RUN_BUDGET).await?;
            let mut endpoint = Endpoint::start(&data_dir, endpoint_port)?;
            register(&client, &node_guard, &endpoint).await?;

            let attaching = reqwest::Client::builder().build()?;
            let repeat = tokio::time::timeout(
                RUN_BUDGET,
                invoke(&attaching, &node_guard, &path_run, Some(request.clone())),
            )
            .await;
            match repeat {
                Ok(Err(error)) => check!(
                    error.contains("409") && error.contains("already invoked"),
                    "the repeat was not refused as an existing invocation: {error}"
                ),
                Ok(Ok(reply)) => {
                    return Err(format!("repeat answered a second execution: {reply}").into())
                }
                Err(error) => return Err(format!("repeat never answered: {error}").into()),
            }
            eprintln!("note: the repeat was refused as an existing invocation, so nothing forked");

            resume(&client, &node_guard, &invocation).await?;
            eprintln!("note: the paused invocation {invocation} was resumed");

            let deadline = Instant::now() + RUN_BUDGET;
            let mut written = at_kill;
            while Instant::now() < deadline && written < Table::ALL.len() {
                tokio::time::sleep(POLL_INTERVAL).await;
                written = snapshots_written();
            }
            check!(written == Table::ALL.len(),
    "the run never finished its merge after the endpoint restarted: {written} of {} snapshots \
     landed (at the kill: {at_kill})\n\
     --- endpoint log ({}):\n{}\n--- node log ({}):\n{}",
    Table::ALL.len(),
    endpoint.log_path.display(),
    endpoint.log(),
    node_guard.log_path.display(),
    std::fs::read_to_string(&node_guard.log_path).map_or(Default::default(), core::convert::identity),);

            endpoint.guard.stop_gracefully();
            let store = Store::open(&data_dir)?;
            let stats = store.stats()?;
            check!(eq; stats.observations, expected_observations as u64,
    "a resumed run replayed a durable write: expected {expected_observations} observations, \
     found {}",
    stats.observations);

            let athletes_snapshot = data_dir.join("out").join("athletes.jsonl");
            let rows = std::fs::read_to_string(&athletes_snapshot)?
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
            check!(eq; rows,
    corpus.athletes.len(),
    "the athletes snapshot holds {rows} rows, not the corpus's {}",
    corpus.athletes.len());

            drop(node_guard);
            let _ = std::fs::remove_dir_all(&root);
            Ok(())
        })
}
#[test]
fn b_restate_server_sigkill_resumes_workflow() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async {
    server_binary().ok_or_else(|| format!("no restate-server found: set {SERVER_ENV} or install one"))?;

    let root = tempfile::TempDir::new()?;
    let data_dir = root.path().join("census");
    std::fs::create_dir_all(&data_dir)?;

    let corpus = synthetic_corpus(SCHOOLS, ATHLETES_PER_SCHOOL)?;
    let expected_observations = corpus.appended_rows();
    {
        let store = Store::open(&data_dir)?;
        corpus.append(&store)?;
    }

    let mut node = Node::start(root.path())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    wait_for_node(&client, &node).await?;

    let endpoint_port = free_port()?;
    let mut endpoint = Endpoint::start(&data_dir, endpoint_port)?;
    register(&client, &node, &endpoint).await?;

    let key = WorkflowIdentity::national(season()?, REVISION, &UsJurisdiction::CENSUS_SCOPE);
    let path_run = format!("Consolidate/{key}/run");
    let request = serde_json::json!({ "tables": [] });

    let submission = {
        let client = client.clone();
        let ingress = node.ingress.clone();
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

    let snapshot_dir = data_dir.join("out");
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
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut written = snapshots_written();
    while Instant::now() < deadline && written < 1 {
        tokio::time::sleep(POLL_INTERVAL).await;
        written = snapshots_written();
    }
    check!(written >= 1,
    "no snapshots written after submitting the run -- the merge did not start");
    check!(written < Table::ALL.len(),
    "the merge completed before we could kill: all {} snapshots written",
    Table::ALL.len());

    endpoint.guard.kill_hard()?;
    check!(snapshots_written() < Table::ALL.len(),
    "the kill landed after completion");
    submission.abort();
    match submission.await {
        Ok(Ok(status)) => eprintln!("note: submission answered {status} before the kill landed"),
        Ok(Err(error)) => eprintln!("note: submission failed as expected: {error}"),
        Err(error) if error.is_cancelled() => {
            eprintln!("note: original ingress caller cancelled after the endpoint kill")
        }
        Err(error) => return Err(format!("submission task panicked: {error}").into()),
    }

    let deadline = Instant::now() + Duration::from_secs(15);
    let original_invocation = paused_invocation(&client, &node, deadline).await?;

    node.restart()?;

    wait_for_node(&client, &node).await?;

    let deadline = Instant::now() + Duration::from_secs(15);
    let resumed_invocation = paused_invocation(&client, &node, deadline).await?;
    check!(eq; original_invocation, resumed_invocation,
    "the invocation ID changed after restart: expected {original_invocation}, found {resumed_invocation}");

    endpoint = Endpoint::start(&data_dir, endpoint_port)?;
    register(&client, &node, &endpoint).await?;
    resume(&client, &node, &resumed_invocation).await?;

    let deadline = Instant::now() + RUN_BUDGET;
    let mut written = snapshots_written();
    while Instant::now() < deadline && written < Table::ALL.len() {
        tokio::time::sleep(POLL_INTERVAL).await;
        written = snapshots_written();
    }
    check!(written == Table::ALL.len(),
    "the merge never finished after server restart: {written} of {} snapshots landed",
    Table::ALL.len());

    drop(node);
    endpoint.guard.stop_gracefully();
    let store = Store::open(&data_dir)?;
    let stats = store.stats()?;
    check!(eq; stats.observations, expected_observations as u64,
    "a resumed run replayed a durable write: expected {expected_observations} observations, \
     found {}",
    stats.observations);

    let athletes_snapshot = data_dir.join("out").join("athletes.jsonl");
    let rows = std::fs::read_to_string(&athletes_snapshot)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    check!(eq; rows,
    corpus.athletes.len(),
    "the athletes snapshot holds {rows} rows, not the corpus's {}",
    corpus.athletes.len());
    Ok(())
        })
}
