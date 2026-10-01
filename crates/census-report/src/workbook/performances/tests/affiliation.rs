use super::{fixture_source, observation, performance_rows};
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Gender, GradYear,
    Mark, SchoolId, SchoolYear, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

const RESULT_DATE: &str = "2026-05-08";

fn season() -> SchoolYear {
    SchoolYear::new(2026).expect("2026 is a season")
}

fn school(store: &Store, state: UsJurisdiction, name: &str) -> SchoolId {
    let (mut row, id) =
        CanonicalSchool::new(state, name, census_domain::model::normalize_name(name));
    row.evidence.push(observation());
    store.append(Table::Schools, &row).unwrap();
    id
}

fn athlete(store: &Store, school: &SchoolId, name: &str) -> AthleteId {
    let mut row = CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        Gender::Boys,
        fixture_source(name),
    );
    let id = row.id.clone();
    row.evidence.push(observation());
    store.append(Table::Athletes, &row).unwrap();
    id
}

fn team(store: &Store, school: &SchoolId) -> TeamId {
    let row = CanonicalTeam {
        id: CanonicalTeam::mint(school, Sport::OutdoorTrack, Gender::Boys, season()),
        school: school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: season(),
        level: None,
        source_identities: Vec::new(),
        evidence: vec![observation()],
        retained_conflicts: Vec::new(),
    };
    let id = row.id.clone();
    store.append(Table::Teams, &row).unwrap();
    id
}

fn performance(store: &Store, athlete: &AthleteId, team: &TeamId) {
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Invitational",
        RESULT_DATE,
        CompetitionLevel::Invitational,
    );
    let meet_id = meet.id.clone();
    meet.evidence.push(observation());
    store.append(Table::Meets, &meet).unwrap();

    let mut event = CanonicalEvent::new(
        &meet_id,
        EventKind::Track400m,
        Gender::Boys,
        None,
        Some("Finals"),
    );
    let event_id = event.id.clone();
    event.evidence.push(observation());
    store.append(Table::Events, &event).unwrap();

    let source_key = format!("test:{}:{RESULT_DATE}", athlete.as_str());
    let row = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            athlete,
            &meet_id,
            &EventKind::Track400m,
            RESULT_DATE,
            &source_key,
        ),
        athlete: athlete.clone(),
        team: team.clone(),
        event: event_id,
        meet: meet_id,
        date: RESULT_DATE.to_string(),
        mark: Mark::TimeSeconds(CentiSeconds::new(4855)),
        wind_mps: None,
        place: Some(1),
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![observation()],
        source_key,
        source_athlete: None,
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Performances, &row).unwrap();
}

fn printed_school(store: &Store) -> String {
    let rows = performance_rows(store, Scope::Core).unwrap();
    assert_eq!(rows.len(), 1, "the fixture writes exactly one result");
    rows.first().expect("the one result").school.clone()
}

fn current_school(store: &Store) -> String {
    let dataset = ExportDataset::load(store).unwrap();
    let derivation = Derivation::of(&dataset, Scope::Core, None);
    derivation
        .athletes()
        .first()
        .expect("the athlete")
        .school
        .as_str()
        .to_string()
}

#[test]
fn a_transfer_keeps_the_result_school_of_the_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let current = school(&store, UsJurisdiction::Wisconsin, "Colby");
    let historical = school(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let athlete = athlete(&store, &current, "Ada");
    let historical_team = team(&store, &historical);
    performance(&store, &athlete, &historical_team);

    let dataset = ExportDataset::load(&store).unwrap();
    assert!(
        dataset.teams.contains_key(historical_team.as_str()),
        "the historical team is persisted"
    );
    assert_eq!(
        current_school(&store),
        current.as_str(),
        "the athlete moved on to Colby"
    );
    assert_eq!(
        printed_school(&store),
        "Abbotsford",
        "the written school is the school of the historical team, not the current one"
    );
}

#[test]
fn an_unresolved_historical_team_keeps_the_school_blank() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let current = school(&store, UsJurisdiction::Wisconsin, "Colby");
    let athlete = athlete(&store, &current, "Ada");
    let unresolved = CanonicalTeam::mint(&current, Sport::OutdoorTrack, Gender::Boys, season());
    performance(&store, &athlete, &unresolved);

    assert_eq!(current_school(&store), current.as_str());
    assert_eq!(
        printed_school(&store),
        "",
        "an unresolved team leaves the school blank instead of rewriting history"
    );
}

#[test]
fn an_out_of_scope_historical_school_keeps_the_school_blank() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let current = school(&store, UsJurisdiction::Wisconsin, "Colby");
    let outside = school(&store, UsJurisdiction::Alaska, "Anchorage");
    let athlete = athlete(&store, &current, "Ada");
    let outside_team = team(&store, &outside);
    performance(&store, &athlete, &outside_team);

    assert_eq!(
        printed_school(&store),
        "",
        "a school outside the run scope prints no name"
    );
}
