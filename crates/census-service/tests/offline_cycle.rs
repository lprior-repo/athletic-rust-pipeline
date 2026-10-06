#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use calamine::Reader;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Id, Mark, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
    TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

const MEET_DATE: &str = "2026-05-02";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const ATHLETES_PER_SCHOOL: usize = 5;

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id("mshsl_results"), MEET_DATE)
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
}

fn build_corpus(schools: usize) -> TestResult<Corpus> {
    let season = SchoolYear::new(2025).ok_or("invalid fixture season")?;
    let mut corpus = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
    };
    for index in 0..schools {
        let name = format!("Offline Cycle School {index}");
        let (mut school, school_id) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            &name,
            normalize_name(&name),
            None,
        );
        school.evidence.push(evidence());
        let team_id = Id::mint("team", &[school_id.as_str(), "track", "m", "2025"]);
        corpus.teams.push(CanonicalTeam {
            id: team_id.clone(),
            school: school_id.clone(),
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
            format!("Offline Cycle Meet {index}"),
            MEET_DATE,
            CompetitionLevel::Invitational,
        );
        let meet_id = meet.id.clone();
        corpus.meets.push(meet);
        corpus.schools.push(school);
        for slot in 0..ATHLETES_PER_SCHOOL {
            let source = SourceIdentity::new(
                SourceNamespace::Other("offline_cycle_fixture".to_string()),
                format!("athlete-{index}-{slot}"),
            );
            let mut athlete = CanonicalAthlete::new(
                &school_id,
                format!("Cycle Runner {index}-{slot}"),
                GradYear::CO2027,
                Gender::Boys,
                source.clone(),
            );
            athlete
                .published_graduations
                .push(census_domain::model::PublishedGraduation {
                    grad_year: GradYear::CO2027,
                    source: SourceRef::id("mshsl_results"),
                });
            let athlete_id = athlete.id.clone();
            let event =
                CanonicalEvent::new(&meet_id, EventKind::Track100m, Gender::Boys, None, None);
            let event_id = event.id.clone();
            corpus.events.push(event);
            corpus.athletes.push(athlete);
            corpus.performances.push(CanonicalPerformance {
                id: CanonicalPerformance::mint(
                    &athlete_id,
                    &meet_id,
                    &EventKind::Track100m,
                    MEET_DATE,
                    &format!("cycle-{index}-{slot}"),
                ),
                athlete: athlete_id,
                team: team_id.clone(),
                event: event_id,
                meet: meet_id.clone(),
                date: MEET_DATE.to_string(),
                mark: Mark::TimeSeconds(
                    CentiSeconds::try_from_seconds_f64(12.0).ok_or("invalid fixture time")?,
                ),
                wind_mps: None,
                place: Some(1),
                heat: None,
                round: None,
                timing: Some(TimingMethod::Fat),
                observed_grade: None,
                evidence: vec![evidence()],
                source_key: format!("cycle-{index}-{slot}"),
                source_athlete: Some(source),
                retained_conflicts: Vec::new(),
            });
        }
    }
    Ok(corpus)
}

fn xlsx_rows(path: &Path, sheet: &str) -> TestResult<usize> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range(sheet)?;
    Ok(range
        .rows()
        .filter(|row| {
            row.iter()
                .any(|cell| !matches!(cell, calamine::Data::Empty))
        })
        .count())
}

fn unique_athlete_ids(path: &Path) -> TestResult<HashSet<String>> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("Athletes")?;
    let mut ids = HashSet::new();
    for row in range.rows().skip(1) {
        if let Some(calamine::Data::String(id)) = row.first() {
            if !id.is_empty() {
                ids.insert(id.clone());
            }
        }
    }
    Ok(ids)
}

fn run_cycle(store: &Path, out: &Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("run")
        .arg("--grad-year")
        .arg("2027")
        .arg("--out")
        .arg(out)
        .arg("--store")
        .arg(store)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
}

fn verify(store: &Path, out: &Path) -> std::io::Result<std::process::ExitStatus> {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("verify")
        .arg("--store")
        .arg(store)
        .arg("--workbook")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

#[test]
fn the_offline_cycle_publishes_each_cohort_athlete_once_in_a_verified_bundle() -> TestResult {
    let tmpdir = TempDir::new()?;
    let data_dir = tmpdir.path().join("store");
    std::fs::create_dir_all(&data_dir)?;

    let corpus = build_corpus(2)?;
    let expected = corpus.athletes.len();
    {
        let store = Store::open(&data_dir)?;
        corpus.append(&store)?;
    }

    let output_path = tmpdir.path().join("publication");
    let output = run_cycle(&data_dir, &output_path)?;
    check!(
        output.status.success(),
        "the offline cycle must exit 0, got {:?}",
        output.status
    );

    let published = census_report::workbook::publication::current_workbook(&output_path)?;

    let athlete_rows = xlsx_rows(&published, "Athletes")?;
    check!(eq; athlete_rows,
    expected + 1,
    "the Athletes sheet carries a header row plus one row per athlete");

    let ids = unique_athlete_ids(&published)?;
    let expected_ids: HashSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.to_string())
        .collect();
    check!(eq; ids, expected_ids,
    "each source-backed cohort subject appears exactly once");

    let status = verify(&data_dir, &published)?;
    check!(
        status.success(),
        "complete publication verification must exit 0, got {status:?}"
    );
    Ok(())
}
