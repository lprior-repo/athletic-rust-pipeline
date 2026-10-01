use super::*;
use crate::CrawlError;
use census_domain::model::{EventKind, Gender, GradYear, Mark, SchoolYear, Sport};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

use super::map::absorb_result_set;
use super::parse::parse_meet_result_files;
use super::results::{Accumulator, Stats};

const TEAMS: &str = include_str!("../../../tests/fixtures/milesplit/wi_teams_index.html");
const ROSTER: &str = include_str!("../../../tests/fixtures/milesplit/wi_roster_52649.html");
const RESULTS_INDEX: &str = include_str!("../../../tests/fixtures/milesplit/oh_results_index.html");

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
    let parsed = parse_roster(ROSTER, teams[0].clone()).unwrap();
    let roster = parsed
        .roster()
        .expect("the fixture has readable roster rows");
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
    let parsed = parse_roster(ROSTER, teams[0].clone()).unwrap();
    let roster = parsed
        .roster()
        .expect("the fixture has readable roster rows");
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, teams_out) = roster_entities(
        roster,
        SchoolYear::new(2026).expect("2026 is a season"),
        "2026-09-20",
        &site,
    );
    assert_eq!(school.name, "Abbotsford");
    assert_eq!(school.city.as_deref(), Some("Abbotsford"));
    assert!(!athletes.is_empty());
    assert!(teams_out.len() >= 2, "indoor/outdoor/XC team variants");
    let aguilera = athletes
        .iter()
        .find(|athlete| athlete.canonical_name == "Julian Aguilera")
        .unwrap();
    assert_eq!(aguilera.grad_year, GradYear::CO2027);
    let observation = aguilera.observed_grades.first().unwrap();
    assert_eq!(observation.grade.get(), 12);
    assert_eq!(observation.grad_year(), Some(GradYear::CO2027));
}

#[test]
fn malformed_html_fails_loudly() {
    assert!(parse_team_index("<html><body>no rows</body></html>").is_err());
    let teams = parse_team_index(TEAMS).unwrap();
    let outcome = parse_roster("<html></html>", teams[0].clone()).unwrap();
    assert!(matches!(
        outcome,
        RosterVerdict::Quarantined {
            reason: RosterQuarantine::UnknownTemplate,
            ..
        }
    ));
}

const OH_TEAMS: &str = include_str!("../../../tests/fixtures/milesplit/oh_teams_index.html");
const OH_ROSTER: &str =
    include_str!("../../../tests/fixtures/milesplit/oh_roster_10002_mason.html");
const OH_RAW: &str =
    include_str!("../../../tests/fixtures/milesplit/oh_meet_770621_rs1321880_raw.html");
const OH_RAW_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw";
const OH_MEET_RESULTS: &str =
    include_str!("../../../tests/fixtures/milesplit/oh_meet_770621_results.html");
const OH_MEET_RESULTS_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results";

#[test]
fn team_index_row_count_pins_a_whole_state_in_one_body() {
    let teams = parse_team_index(OH_TEAMS).unwrap();
    assert_eq!(teams.len(), 977);
    let unique: std::collections::HashSet<&str> =
        teams.iter().map(|team| team.id.as_str()).collect();
    assert_eq!(unique.len(), teams.len(), "team ids are the index key");
    assert!(teams
        .iter()
        .all(|team| team.url.starts_with("https://oh.milesplit.com/teams/")));
    let mason = teams.iter().find(|team| team.id == "10002").expect("Mason");
    assert!(
        mason.city_state.contains("OH"),
        "city_state: {}",
        mason.city_state
    );
}

#[test]
fn oh_roster_pins_319_graded_rows_and_96_class_of_2027() {
    let teams = parse_team_index(OH_TEAMS).unwrap();
    let mason = teams
        .iter()
        .find(|team| team.id == "10002")
        .unwrap()
        .clone();
    let parsed = parse_roster(OH_ROSTER, mason).unwrap();
    let roster = parsed
        .roster()
        .expect("the fixture has readable roster rows");
    assert_eq!(roster.athletes.len(), 318);
    let co2027 = roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count();
    assert_eq!(co2027, 96);
    assert!(roster
        .athletes
        .iter()
        .all(|athlete| (2027..=2030).contains(&athlete.grad_year.get())));
}

#[test]
fn raw_result_set_body_pins_80_rows_in_two_sections() {
    let page = parse_raw(OH_RAW, OH_RAW_URL).unwrap();
    assert_eq!(page.meet.rows_parsed, 80);
    assert_eq!(page.meet.rows_skipped, 0);
    assert!(page.skipped.is_empty(), "skipped: {:?}", page.skipped);
    assert_eq!(page.meet.name, "Beaver Eastern Invite");
    assert_eq!(page.meet.date, "2026-09-19");
    assert_eq!(page.meet.end_date.as_deref(), Some("2026-09-19"));
    assert_eq!(page.sport, Some(Sport::CrossCountry));
    assert_eq!(page.region.as_deref(), Some("OH"));
    assert_eq!(
        page.school_year,
        SchoolYear::new(2026).expect("2026 is a season")
    );
    assert_eq!(page.meet.events.len(), 2);
    let boys = &page.meet.events[0];
    assert_eq!(boys.label, "Boys Middle School 3000 Meter");
    assert_eq!(boys.gender, Gender::Boys);
    assert_eq!(boys.kind, EventKind::CrossCountry);
    assert_eq!(boys.rows.len(), 40);
    let girls = &page.meet.events[1];
    assert_eq!(girls.label, "Girls Middle School 3000 Meter");
    assert_eq!(girls.gender, Gender::Girls);
    assert_eq!(girls.rows.len(), 40);
    let first = &boys.rows[0];
    assert_eq!(first.place, Some(1));
    assert_eq!(first.name, "Jeydyn Fields");
    assert_eq!(first.school, "Jackson");
    assert_eq!(first.grade, None);
    assert!(matches!(first.mark, Mark::TimeSeconds(_)));
}
mod parsing;
mod roster;

#[test]
fn rejected_rows_keep_their_provider_identity_and_exact_utf8_span() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
    let body = format!(
        "é\n{}",
        ROSTER.replacen("column-grad-year\">2027", "column-grad-year\">invalid", 1)
    );
    let RosterVerdict::Partial { roster, rejected } = parse_roster(&body, team).unwrap() else {
        panic!("one malformed graduation year must not discard the readable rows");
    };
    assert_eq!(roster.athletes.len(), 24);
    assert_eq!(rejected.len(), 1);
    let rejection = &rejected[0];
    assert_eq!(rejection.kind, RosterRejectionKind::InvalidGraduationYear);
    let locator = rejection.row;
    let span = &body[locator.byte_offset..locator.byte_offset + locator.byte_length];
    assert!(span.contains("column-grad-year\">invalid"));
    let id = rejection.athlete_id.as_deref().unwrap();
    assert!(span.contains(&format!("/athletes/{id}-")));
    assert!(roster
        .athletes
        .iter()
        .all(|athlete| athlete.athlete_id != id));
    assert_eq!(
        locator.ordinal as usize + 1,
        body[..locator.byte_offset]
            .matches("<li class=\"athlete-row data-row\">")
            .count()
    );
}

#[test]
fn a_missing_name_does_not_erase_a_readable_provider_identity() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
    let body = "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/42-empty\"></a></li>";
    let RosterVerdict::Quarantined { reason, rejected } = parse_roster(body, team).unwrap() else {
        panic!("an unnamed athlete cannot be published");
    };
    assert_eq!(reason, RosterQuarantine::NoReadableRows);
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].kind, RosterRejectionKind::MissingName);
    assert_eq!(rejected[0].athlete_id.as_deref(), Some("42"));
}

#[test]
fn an_empty_known_roster_is_a_named_gap_not_negative_identity_evidence() {
    let team = parse_team_index(TEAMS).unwrap().remove(0);
    let outcome = parse_roster("<ul id=\"rosterDataset\"></ul>", team).unwrap();
    assert!(matches!(outcome, RosterVerdict::Quarantined {
        reason: RosterQuarantine::NoReadableRows, rejected
    } if rejected.is_empty()));
}
