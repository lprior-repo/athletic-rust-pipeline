use super::*;
use census_domain::UsJurisdiction;

const TEAMS: &str = include_str!("../../../tests/fixtures/milesplit/wi_teams_index.html");
const ROSTER: &str = include_str!("../../../tests/fixtures/milesplit/wi_roster_52649.html");

#[test]
fn parses_team_index_rows() {
    let teams = parse_team_index(TEAMS).unwrap();
    assert_eq!(teams.len(), 40);
    assert_eq!(teams[0].id, "52649");
    assert_eq!(teams[0].name, "Abbotsford");
    assert_eq!(teams[0].city_state, "ABBOTSFORD, WI, USA");
}

#[test]
fn parses_roster_rows_with_grad_year_and_seasons() {
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
    assert_eq!(roster.athletes.len(), 25);
    let first = &roster.athletes[0];
    assert_eq!(first.name, "Julian Aguilera");
    assert_eq!(first.roster_name, "Aguilera, Julian");
    assert_eq!(first.grad_year, GradYear::CO2027);
    assert_eq!(first.gender, Gender::Boys);
    assert_eq!(first.athlete_id, "14399169");
    assert!(first
        .profile_url
        .ends_with("/athletes/14399169-julian-aguilera"));
    // This athlete's roster row carries no season flags (matched athlete, no imported results).
    assert!(first.sports().is_empty());
    let all_sports = roster
        .athletes
        .iter()
        .find(|athlete| athlete.roster_name.starts_with("Altamirano"))
        .expect("Altamirano row present in fixture");
    assert_eq!(
        all_sports.sports(),
        vec![Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
    );
}

#[test]
fn roster_entities_are_canonical_and_source_independent() {
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster(ROSTER, teams[0].clone()).unwrap();
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, teams_out) =
        roster_entities(&roster, SchoolYear(2026), "2026-09-20", &site);
    assert_eq!(school.name, "Abbotsford");
    assert_eq!(school.city.as_deref(), Some("Abbotsford"));
    assert!(!athletes.is_empty());
    assert!(teams_out.len() >= 2, "indoor/outdoor/XC team variants");
    let aguilera = athletes
        .iter()
        .find(|athlete| athlete.canonical_name == "Julian Aguilera")
        .unwrap();
    assert_eq!(aguilera.grad_year, GradYear::CO2027);
    // Grade observed on a 2026-27 roster is 12 for a 2027 graduate.
    let observation = aguilera.observed_grades.first().unwrap();
    assert_eq!(observation.grade.get(), 12);
    assert_eq!(observation.grad_year(), GradYear::CO2027);
    assert_eq!(
        aguilera.id,
        CanonicalAthlete::mint(
            &school.id,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys
        )
    );
}

#[test]
fn malformed_html_fails_loudly() {
    assert!(parse_team_index("<html><body>no rows</body></html>").is_err());
    let teams = parse_team_index(TEAMS).unwrap();
    let roster = parse_roster("<html></html>", teams[0].clone()).unwrap();
    assert!(
        roster.athletes.is_empty(),
        "empty roster is data, not an error"
    );
}
