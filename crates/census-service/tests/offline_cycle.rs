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
const SEASON: SchoolYear = SchoolYear::new(2025).expect("2025 is a season");
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

fn build_corpus(schools: usize) -> Corpus {
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
        let (mut school, school_id) =
            CanonicalSchool::new(UsJurisdiction::Wisconsin, &name, &normalize_name(&name));
        school.evidence.push(evidence());
        let team_id = Id::mint("team", &[school_id.as_str(), "track", "m", "2025"]);
        corpus.teams.push(CanonicalTeam {
            id: team_id.clone(),
            school: school_id.clone(),
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            school_year: SEASON,
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
            let athlete = CanonicalAthlete::new(
                &school_id,
                format!("Cycle Runner {index}-{slot}"),
                GradYear::CO2027,
                Gender::Boys,
                source.clone(),
            );
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
                    CentiSeconds::try_from_seconds_f64(12.0).expect("in range"),
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
    corpus
}

fn xlsx_rows(path: &Path, sheet: &str) -> Option<usize> {
    let mut book = calamine::open_workbook_auto(path).ok()?;
    let range = book.worksheet_range(sheet).ok()?;
    Some(
        range
            .rows()
            .filter(|row| {
                row.iter()
                    .any(|cell| !matches!(cell, calamine::Data::Empty))
            })
            .count(),
    )
}

fn unique_athlete_ids(path: &Path) -> Option<HashSet<String>> {
    let mut book = calamine::open_workbook_auto(path).ok()?;
    let range = book.worksheet_range("Athletes").ok()?;
    let mut ids = HashSet::new();
    for row in range.rows().skip(1) {
        if let Some(calamine::Data::String(id)) = row.first() {
            if !id.is_empty() {
                ids.insert(id.clone());
            }
        }
    }
    Some(ids)
}

fn run_cycle(store: &Path, out: &Path) -> std::process::Output {
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
        .expect("run the offline cycle")
}

fn verify(store: &Path, out: &Path) -> std::process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("verify")
        .arg("--store")
        .arg(store)
        .arg("--workbook")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run complete publication verification")
}

#[test]
fn the_offline_cycle_publishes_each_cohort_athlete_once_in_a_verified_bundle() {
    let tmpdir = TempDir::new().expect("create temp dir");
    let data_dir = tmpdir.path().join("store");
    std::fs::create_dir_all(&data_dir).expect("create store dir");

    let corpus = build_corpus(2);
    let expected = corpus.athletes.len();
    {
        let store = Store::open(&data_dir).expect("open store");
        corpus.append(&store);
    }

    let output_path = tmpdir.path().join("publication");
    let output = run_cycle(&data_dir, &output_path);
    assert!(
        output.status.success(),
        "the offline cycle must exit 0, got {:?}",
        output.status
    );

    let published = census_report::workbook::publication::current_workbook(&output_path)
        .expect("complete cycle publication");

    let athlete_rows = xlsx_rows(&published, "Athletes").expect("read the Athletes sheet");
    assert_eq!(
        athlete_rows,
        expected + 1,
        "the Athletes sheet carries a header row plus one row per athlete"
    );

    let ids = unique_athlete_ids(&published).expect("read the athlete ids");
    let expected_ids: HashSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.to_string())
        .collect();
    assert_eq!(
        ids, expected_ids,
        "each source-backed cohort subject appears exactly once"
    );

    let status = verify(&data_dir, &published);
    assert!(
        status.success(),
        "complete publication verification must exit 0, got {status:?}"
    );
}
