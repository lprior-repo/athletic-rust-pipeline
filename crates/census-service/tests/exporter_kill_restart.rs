#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use calamine::Reader;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Id, Mark, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
    TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::HashSet;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const MEET_DATE: &str = "2026-05-02";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id("mshsl_results"), MEET_DATE)
}
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
    fn append(&self, store: &Store) -> TestResult {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }
    fn count(&self) -> usize {
        self.schools.len()
            + self.teams.len()
            + self.athletes.len()
            + self.meets.len()
            + self.events.len()
            + self.performances.len()
    }
}

fn build_corpus(n: usize) -> TestResult<Corpus> {
    let season = SchoolYear::new(2025).ok_or("invalid fixture season")?;
    let mut c = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
    };
    for i in 0..n {
        let name = format!("KillRestart School {i}");
        let (mut school, sid) =
            CanonicalSchool::new(UsJurisdiction::Wisconsin, &name, normalize_name(&name));
        school.evidence.push(evidence());
        let tid = Id::mint("team", &[sid.as_str(), "track", "m", "2025"]);
        c.teams.push(CanonicalTeam {
            id: tid.clone(),
            school: sid.clone(),
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            school_year: season,
            level: None,
            source_identities: Vec::new(),
            evidence: vec![evidence()],
            retained_conflicts: Vec::new(),
        });
        let meet = CanonicalMeet::new(
            Some(UsJurisdiction::Wisconsin),
            format!("KillRestart Meet {i}"),
            MEET_DATE,
            CompetitionLevel::Invitational,
        );
        let mid = meet.id.clone();
        c.meets.push(meet);
        c.schools.push(school);
        for s in 0..ATH_PER_SCHOOL {
            let source = SourceIdentity::new(
                SourceNamespace::Other("fixture".to_string()),
                format!("athlete-{i}-{s}"),
            );
            let mut a = CanonicalAthlete::new(
                &sid,
                format!("Killer {i}-{s}"),
                GradYear::CO2027,
                Gender::Boys,
                source.clone(),
            );
            a.published_graduations
                .push(census_domain::model::PublishedGraduation {
                    grad_year: GradYear::CO2027,
                    source: SourceRef::id("mshsl_results"),
                });
            let aid = a.id.clone();
            let ev = CanonicalEvent::new(&mid, EventKind::Track100m, Gender::Boys, None, None);
            let eid = ev.id.clone();
            c.events.push(ev);
            c.athletes.push(a);
            let pid = CanonicalPerformance::mint(
                &aid,
                &mid,
                &EventKind::Track100m,
                MEET_DATE,
                &format!("kill-{i}-{s}"),
            );
            c.performances.push(CanonicalPerformance {
                id: pid,
                athlete: aid,
                team: tid.clone(),
                event: eid,
                meet: mid.clone(),
                date: MEET_DATE.to_string(),
                mark: Mark::TimeSeconds(
                    CentiSeconds::try_from_seconds_f64(12.0).ok_or("invalid fixture time")?,
                ),
                wind_mps: None,
                place: Some(1),
                heat: None,
                round: None,
                timing: Some(TimingMethod::Fat),
                observed_grade: Some(Grade::new(11).ok_or("invalid fixture grade")?),
                evidence: vec![evidence()],
                source_key: format!("kill-{i}-{s}"),
                source_athlete: Some(source),
                retained_conflicts: Vec::new(),
            });
        }
    }
    Ok(c)
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn kill_and_reap(&mut self) -> TestResult {
        self.child.kill()?;
        let status = self.child.wait()?;
        check!(eq; status.signal(),
        Some(9),
        "child must die from SIGKILL, got {:?}",
        status);
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_workbook(data_dir: &Path, out_path: &Path) -> TestResult<ChildGuard> {
    let binary = env!("CARGO_BIN_EXE_census-service");
    let child = Command::new(binary)
        .arg("workbook")
        .arg("--grad-year")
        .arg("2027")
        .arg("--out")
        .arg(out_path)
        .arg("--store")
        .arg(data_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(ChildGuard { child })
}

fn xlsx_rows(path: &Path, sheet: &str) -> TestResult<usize> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range(sheet)?;
    Ok(range
        .rows()
        .filter(|r| r.iter().any(|c| !matches!(c, calamine::Data::Empty)))
        .count())
}

fn unique_athlete_ids(path: &Path) -> TestResult<HashSet<String>> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("Athletes")?;
    let mut ids = HashSet::new();
    for row in range.rows().skip(1) {
        match row.first() {
            Some(calamine::Data::String(s)) if !s.is_empty() => {
                ids.insert(s.clone());
            }
            _ => {}
        }
    }
    Ok(ids)
}

fn verify_cli(store: &Path, xlsx: &Path) -> TestResult {
    let binary = env!("CARGO_BIN_EXE_census-service");
    let status = Command::new(binary)
        .arg("verify")
        .arg("--store")
        .arg(store)
        .arg("--workbook")
        .arg(xlsx)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    check!(status.success(), "verify must exit 0, got {:?}", status);
    Ok(())
}

#[test]
fn a_workbook_export_interrupted_by_sigkill_rebuilds_completely_on_restart() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async {
            let tmpdir = TempDir::new()?;
            let data_dir = tmpdir.path().join("store");
            std::fs::create_dir_all(&data_dir)?;

            let corpus = build_corpus(200)?;
            let expected = corpus.count();
            {
                let store = Store::open(&data_dir)?;
                corpus.append(&store)?;
            }

            let output_path = tmpdir.path().join("publication");
            let mut child = spawn_workbook(&data_dir, &output_path)?;

            let deadline = Instant::now() + Duration::from_secs(120);
            let mut killed = false;
            while Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(200));
                if let Ok(entries) = std::fs::read_dir(&output_path) {
                    let rendering = entries.filter_map(Result::ok).any(|entry| {
                        entry.file_name().to_string_lossy().starts_with(".staging.")
                            && entry.path().join("frozen-input.json").is_file()
                    });
                    if rendering {
                        child.kill_and_reap()?;
                        killed = true;
                        break;
                    }
                }
            }
            check!(
                killed,
                "an uncommitted frozen input never appeared before deadline"
            );

            check!(
                !output_path.join("current").exists(),
                "interrupted generation must not become public"
            );

            {
                let s = Store::open(&data_dir)?;
                let _ = s.stats()?;
            }

            let mut restart = spawn_workbook(&data_dir, &output_path)?;
            let st = restart.child.wait()?;
            check!(st.success(), "restarted workbook must exit 0, got {:?}", st);

            let published = census_report::workbook::publication::current_workbook(&output_path)?;
            check!(eq; xlsx_rows(&published, "Athletes")?,
    corpus.athletes.len() + 1);
            let expected_ids: HashSet<_> = corpus
                .athletes
                .iter()
                .map(|athlete| athlete.id.to_string())
                .collect();
            check!(eq; unique_athlete_ids(&published)?,
    expected_ids);
            verify_cli(&data_dir, &published)?;

            let store_after = Store::open(&data_dir)?;
            let stats = store_after.stats()?;
            check!(eq; stats.observations,
    u64::try_from(expected)?,
    "store must hold all observations: expected={}, found={}",
    expected,
    stats.observations);
            Ok(())
        })
}

#[test]
fn b_workbook_without_interrupt_exits_cleanly() -> TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async {
            let tmpdir = TempDir::new()?;
            let data_dir = tmpdir.path().join("store");
            std::fs::create_dir_all(&data_dir)?;

            let corpus = build_corpus(20)?;
            {
                let store = Store::open(&data_dir)?;
                corpus.append(&store)?;
            }
            let output_path = tmpdir.path().join("publication");
            let mut child = spawn_workbook(&data_dir, &output_path)?;
            let status = child.child.wait()?;
            check!(status.success(), "must exit 0, got {:?}", status);
            let published = census_report::workbook::publication::current_workbook(&output_path)?;
            check!(eq; xlsx_rows(&published, "Athletes")?,
    corpus.athletes.len() + 1);
            verify_cli(&data_dir, &published)?;
            Ok(())
        })
}
