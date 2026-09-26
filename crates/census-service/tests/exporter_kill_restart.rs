use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Id, Mark, SchoolId, SchoolYear, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::HashSet;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use calamine::Reader;
use tempfile::TempDir;

const MEET_DATE: &str = "2026-05-02";
const SEASON: SchoolYear = SchoolYear::new(2025).expect("2025 is a season");
fn evidence() -> Evidence { Evidence::parsed(SourceRef::id("mshsl_results"), MEET_DATE) }
const ATH_PER_SCHOOL: usize = 100;

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
        store.append_many(Table::Performances, &self.performances).unwrap();
    }
    fn count(&self) -> usize {
        self.schools.len() + self.teams.len() + self.athletes.len()
            + self.meets.len() + self.events.len() + self.performances.len()
    }
}

fn build_corpus(n: usize) -> Corpus {
    let mut c = Corpus { schools: Vec::new(), teams: Vec::new(), athletes: Vec::new(),
        meets: Vec::new(), events: Vec::new(), performances: Vec::new() };
    for i in 0..n {
        let name = format!("KillRestart School {i}");
        let (mut school, sid) = CanonicalSchool::new(UsJurisdiction::Wisconsin, &name, &normalize_name(&name));
        school.evidence.push(evidence());
        let tid = Id::mint("team", &[sid.as_str(), "track", "m", "2025"]);
        c.teams.push(CanonicalTeam { id: tid.clone(), school: sid.clone(), sport: Sport::OutdoorTrack,
            gender: Gender::Boys, school_year: SEASON, level: None,
            source_identities: Vec::new(), evidence: vec![evidence()], retained_conflicts: Vec::new() });
        let meet = CanonicalMeet::new(Some(UsJurisdiction::Wisconsin), format!("KillRestart Meet {i}"), MEET_DATE, CompetitionLevel::Invitational);
        let mid = meet.id.clone();
        c.meets.push(meet); c.schools.push(school);
        for s in 0..ATH_PER_SCHOOL {
            let a = CanonicalAthlete::new(&sid, format!("Killer {i}-{s}"), GradYear::CO2027, Gender::Boys);
            let aid = a.id.clone();
            let ev = CanonicalEvent::new(&mid, EventKind::Track100m, Gender::Boys, None, None);
            let eid = ev.id.clone();
            c.events.push(ev); c.athletes.push(a);
            let pid = CanonicalPerformance::mint(&aid, &mid, &EventKind::Track100m, MEET_DATE, &format!("kill-{i}-{s}"));
            c.performances.push(CanonicalPerformance { id: pid, athlete: aid, team: tid.clone(), event: eid,
                meet: mid.clone(), date: MEET_DATE.to_string(),
                mark: Mark::TimeSeconds(CentiSeconds::try_from_seconds_f64(12.0).expect("in range")),
                wind_mps: None, place: Some(1), heat: None, round: None, timing: Some(TimingMethod::Fat),
                observed_grade: Some(Grade::new(11).unwrap()), evidence: vec![evidence()],
                source_key: format!("kill-{i}-{s}"), source_athlete: None, retained_conflicts: Vec::new() });
        }
    }
    c
}

struct ChildGuard { child: Child }

impl ChildGuard {
    fn kill_and_reap(&mut self) {
        self.child.kill().expect("SIGKILL child");
        let status = self.child.wait().expect("reap killed child");
        assert_eq!(status.signal(), Some(9), "child must die from SIGKILL, got {:?}", status);
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); }
}

fn spawn_workbook(data_dir: &Path, out_path: &Path) -> ChildGuard {
    let binary = env!("CARGO_BIN_EXE_census-service");
    let child = Command::new(binary).arg("workbook").arg("--grad-year").arg("2027")
        .arg("--out").arg(out_path).arg("--store").arg(data_dir)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().expect("spawn census-service workbook");
    ChildGuard { child }
}

fn xlsx_rows(path: &Path, sheet: &str) -> Option<usize> {
    let mut book = calamine::open_workbook_auto(path).ok()?;
    let range = book.worksheet_range(sheet).ok()?;
    Some(range.rows().filter(|r| r.iter().any(|c| !matches!(c, calamine::Data::Empty))).count())
}

fn unique_athlete_ids(path: &Path) -> Option<HashSet<String>> {
    let mut book = calamine::open_workbook_auto(path).ok()?;
    let range = book.worksheet_range("Athletes").ok()?;
    let mut ids = HashSet::new();
    for row in range.rows().skip(1) {
        if let Some(cell) = row.first() {
            if let calamine::Data::String(ref s) = cell {
                if !s.is_empty() {
                    ids.insert(s.clone());
                }
            }
        }
    }
    Some(ids)
}


fn verify_cli(store: &Path, xlsx: &Path) {
    let binary = env!("CARGO_BIN_EXE_census-service");
    let status = Command::new(binary).arg("verify").arg("--store").arg(store)
        .arg("--workbook").arg(xlsx).stdout(Stdio::null()).stderr(Stdio::null())
        .status().expect("spawn verify");
    assert!(status.success(), "verify must exit 0, got {:?}", status);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_workbook_export_interrupted_by_sigkill_rebuilds_completely_on_restart() {
    let tmpdir = TempDir::new().expect("create temp dir");
    let data_dir = tmpdir.path().join("store");
    std::fs::create_dir_all(&data_dir).expect("create store dir");

    let corpus = build_corpus(200);
    let expected = corpus.count();
    { let store = Store::open(&data_dir).expect("open store"); corpus.append(&store); }

    let output_path = tmpdir.path().join("out.xlsx");
    let mut child = spawn_workbook(&data_dir, &output_path);

    let deadline = Instant::now() + Duration::from_secs(120);
    let mut killed = false;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
        if output_path.exists() {
            if let Ok(meta) = std::fs::metadata(&output_path) {
                if meta.len() > 0 { child.kill_and_reap(); killed = true; break; }
            }
        }
    }
    assert!(killed, "workbook output never appeared with non-zero bytes before deadline");

    let orig_len = std::fs::metadata(&output_path).expect("stat output").len();

    { let s = Store::open(&data_dir).expect("reopen store"); let _ = s.stats().expect("stats"); }

    let mut restart = spawn_workbook(&data_dir, &output_path);
    let st = restart.child.wait().expect("wait restart");
    assert!(st.success(), "restarted workbook must exit 0, got {:?}", st);

    assert!(output_path.exists(), "output must exist after restart");
    let final_meta = std::fs::metadata(&output_path).expect("stat final");
    assert!(final_meta.len() > orig_len, "restarted workbook larger than interrupted: {} vs {}", final_meta.len(), orig_len);

    assert!(xlsx_rows(&output_path, "Athletes").is_some(), "must have Athletes sheet");
    let athletes_rows = xlsx_rows(&output_path, "Athletes").expect("Athletes rows");
    assert!(athletes_rows > 200, "Athletes sheet must have data rows, found {athletes_rows}");

    let unique = unique_athlete_ids(&output_path).expect("unique IDs");
    assert!(unique.len() >= 200, ">=200 unique athlete IDs, found {}", unique.len());

    verify_cli(&data_dir, &output_path);

    let store_after = Store::open(&data_dir).expect("reopen store after restart");
    let stats = store_after.stats().expect("final stats");
    assert_eq!(stats.observations, expected as u64, "store must hold all observations: expected={}, found={}", expected, stats.observations);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn b_workbook_without_interrupt_exits_cleanly() {
    let tmpdir = TempDir::new().expect("create temp dir");
    let data_dir = tmpdir.path().join("store");
    std::fs::create_dir_all(&data_dir).expect("create store dir");

    let corpus = build_corpus(20);
    { let store = Store::open(&data_dir).expect("open"); corpus.append(&store); }
    let output_path = tmpdir.path().join("clean.xlsx");
    let mut child = spawn_workbook(&data_dir, &output_path);
    let status = child.child.wait().expect("wait clean");
    assert!(status.success(), "must exit 0, got {:?}", status);
    assert!(output_path.exists(), "output must exist");
    let len = std::fs::metadata(&output_path).expect("stat").len();
    assert!(len > 0, "output must have non-zero size: {len}");

    assert!(xlsx_rows(&output_path, "Athletes").is_some(), "must have Athletes sheet");
    verify_cli(&data_dir, &output_path);
}
