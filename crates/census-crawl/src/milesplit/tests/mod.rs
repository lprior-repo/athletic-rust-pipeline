use super::*;
use crate::CrawlError;
use census_domain::model::{
    Confidence, EventKind, Gender, GradYear, Mark, PublishedGraduation, SchoolYear, SourceRef,
    Sport,
};
use census_domain::UsJurisdiction;

use super::parse::parse_meet_result_files;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TEAMS: &str = include_str!("../../../tests/fixtures/milesplit/wi_teams_index.html");
const ROSTER: &str = include_str!("../../../tests/fixtures/milesplit/wi_roster_52649.html");
const RESULTS_INDEX: &str = include_str!("../../../tests/fixtures/milesplit/oh_results_index.html");

#[test]
fn parses_team_index_rows() -> TestResult {
    let teams = parse_team_index(TEAMS)?;
    check!(eq; teams.len(), 40);
    check!(eq; teams[0].id, "52649");
    check!(eq; teams[0].name, "Abbotsford");
    check!(eq; teams[0].city_state, "ABBOTSFORD, WI, USA");
    Ok(())
}

#[test]
fn parses_roster_rows_with_grad_year_and_seasons() -> TestResult {
    let teams = parse_team_index(TEAMS)?;
    let parsed = parse_roster(ROSTER, teams[0].clone())?;
    let roster = parsed
        .roster()
        .ok_or("the fixture has readable roster rows")?;
    check!(eq; roster.athletes.len(), 25);
    let first = &roster.athletes[0];
    check!(eq; first.name, "Julian Aguilera");
    check!(eq; first.roster_name, "Aguilera, Julian");
    check!(eq; first.grad_year, GradYear::CO2027);
    check!(eq; first.gender, Gender::Boys);
    check!(eq; first.athlete_id, "14399169");
    check!(first
        .profile_url
        .ends_with("/athletes/14399169-julian-aguilera"));
    check!(first.sports().is_empty());
    let all_sports = roster
        .athletes
        .iter()
        .find(|athlete| athlete.roster_name.starts_with("Altamirano"))
        .ok_or("Altamirano row present in fixture")?;
    check!(eq;
        all_sports.sports(),
        vec![Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
    );
    Ok(())
}

#[test]
fn roster_entities_are_canonical_and_source_independent() -> TestResult {
    let teams = parse_team_index(TEAMS)?;
    let parsed = parse_roster(ROSTER, teams[0].clone())?;
    let roster = parsed
        .roster()
        .ok_or("the fixture has readable roster rows")?;
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, teams_out) = roster_entities(
        roster,
        SchoolYear::new(2026).ok_or("2026 is a season")?,
        "2026-09-20",
        &site,
    );
    check!(eq; school.name, "Abbotsford");
    check!(eq; school.city.as_deref(), Some("Abbotsford"));
    check!(!athletes.is_empty());
    check!(teams_out.len() >= 2, "indoor/outdoor/XC team variants");
    let aguilera = athletes
        .iter()
        .find(|athlete| athlete.canonical_name == "Julian Aguilera")
        .ok_or("Julian Aguilera athlete")?;
    check!(eq; aguilera.grad_year, GradYear::CO2027);
    check!(eq; aguilera.observed_grades, Vec::new());
    check!(eq;
        aguilera.published_graduations,
        vec![PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::new(
                site.source_id(),
                Some(format!("{}/roster", roster.team.url)),
            ),
        }]
    );
    check!(eq; aguilera.derived_cohort_confidence(), Some(Confidence::HIGH));
    Ok(())
}

#[test]
fn malformed_html_fails_loudly() -> TestResult {
    check!(parse_team_index("<html><body>no rows</body></html>").is_err());
    let teams = parse_team_index(TEAMS)?;
    let outcome = parse_roster("<html></html>", teams[0].clone())?;
    check!(matches!(
        outcome,
        RosterVerdict::Quarantined {
            reason: RosterQuarantine::UnknownTemplate,
            ..
        }
    ));
    Ok(())
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
fn team_index_row_count_pins_a_whole_state_in_one_body() -> TestResult {
    let teams = parse_team_index(OH_TEAMS)?;
    check!(eq; teams.len(), 977);
    let unique: std::collections::HashSet<&str> =
        teams.iter().map(|team| team.id.as_str()).collect();
    check!(eq; unique.len(), teams.len(), "team ids are the index key");
    check!(teams
        .iter()
        .all(|team| team.url.starts_with("https://oh.milesplit.com/teams/")));
    let mason = teams
        .iter()
        .find(|team| team.id == "10002")
        .ok_or("Mason")?;
    check!(
        mason.city_state.contains("OH"),
        "city_state: {}",
        mason.city_state
    );
    Ok(())
}

#[test]
fn oh_roster_pins_319_graded_rows_and_96_class_of_2027() -> TestResult {
    let teams = parse_team_index(OH_TEAMS)?;
    let mason = teams
        .iter()
        .find(|team| team.id == "10002")
        .ok_or("Mason")?
        .clone();
    let parsed = parse_roster(OH_ROSTER, mason)?;
    let roster = parsed
        .roster()
        .ok_or("the fixture has readable roster rows")?;
    check!(eq; roster.athletes.len(), 318);
    let co2027 = roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count();
    check!(eq; co2027, 96);
    check!(roster
        .athletes
        .iter()
        .all(|athlete| (2027..=2030).contains(&athlete.grad_year.get())));
    Ok(())
}

#[test]
fn raw_result_set_body_pins_80_rows_in_two_sections() -> TestResult {
    let page = parse_raw(OH_RAW, OH_RAW_URL)?;
    check!(eq; page.meet.rows_parsed, 80);
    check!(eq; page.meet.rows_skipped, 0);
    check!(page.skipped.is_empty(), "skipped: {:?}", page.skipped);
    check!(eq; page.meet.name, "Beaver Eastern Invite");
    check!(eq; page.meet.date, "2026-09-19");
    check!(eq; page.meet.end_date.as_deref(), Some("2026-09-19"));
    check!(eq; page.sport, Some(Sport::CrossCountry));
    check!(eq; page.region.as_deref(), Some("OH"));
    check!(eq;
        page.school_year,
        SchoolYear::new(2026).ok_or("2026 is a season")?
    );
    check!(eq; page.meet.events.len(), 2);
    let boys = &page.meet.events[0];
    check!(eq; boys.label, "Boys Middle School 3000 Meter");
    check!(eq; boys.gender, Gender::Boys);
    check!(eq; boys.kind, EventKind::CrossCountry);
    check!(eq; boys.rows.len(), 40);
    let girls = &page.meet.events[1];
    check!(eq; girls.label, "Girls Middle School 3000 Meter");
    check!(eq; girls.gender, Gender::Girls);
    check!(eq; girls.rows.len(), 40);
    let first = &boys.rows[0];
    check!(eq; first.place, Some(1));
    check!(eq; first.name, "Jeydyn Fields");
    check!(eq; first.school, "Jackson");
    check!(eq; first.grade, None);
    check!(matches!(first.mark, Mark::TimeSeconds(_)));
    Ok(())
}
mod parsing;
mod roster;
mod roster_entities;

#[test]
fn rejected_rows_keep_their_provider_identity_and_exact_utf8_span() -> TestResult {
    let team = parse_team_index(TEAMS)?.remove(0);
    let body = format!(
        "é\n{}",
        ROSTER.replacen("column-grad-year\">2027", "column-grad-year\">invalid", 1)
    );
    let RosterVerdict::Partial { roster, rejected } = parse_roster(&body, team)? else {
        return Err("one malformed graduation year must not discard the readable rows".into());
    };
    check!(eq; roster.athletes.len(), 24);
    check!(eq; rejected.len(), 1);
    let rejection = &rejected[0];
    check!(eq; rejection.kind, RosterRejectionKind::InvalidGraduationYear);
    let locator = rejection.row;
    let span = &body[locator.byte_offset..locator.byte_offset + locator.byte_length];
    check!(span.contains("column-grad-year\">invalid"));
    let id = rejection
        .athlete_id
        .as_deref()
        .ok_or("rejected athlete identity")?;
    check!(span.contains(&format!("/athletes/{id}-")));
    check!(roster
        .athletes
        .iter()
        .all(|athlete| athlete.athlete_id != id));
    check!(eq;
        locator.ordinal as usize + 1,
        body[..locator.byte_offset]
            .matches("<li class=\"athlete-row data-row\">")
            .count()
    );
    Ok(())
}

#[test]
fn a_missing_name_does_not_erase_a_readable_provider_identity() -> TestResult {
    let team = parse_team_index(TEAMS)?.remove(0);
    let body = "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/42-empty\"></a></li>";
    let RosterVerdict::Quarantined { reason, rejected } = parse_roster(body, team)? else {
        return Err("an unnamed athlete cannot be published".into());
    };
    check!(eq; reason, RosterQuarantine::NoReadableRows);
    check!(eq; rejected.len(), 1);
    check!(eq; rejected[0].kind, RosterRejectionKind::MissingName);
    check!(eq; rejected[0].athlete_id.as_deref(), Some("42"));
    Ok(())
}

#[test]
fn an_empty_known_roster_is_a_named_gap_not_negative_identity_evidence() -> TestResult {
    let team = parse_team_index(TEAMS)?.remove(0);
    let outcome = parse_roster("<ul id=\"rosterDataset\"></ul>", team)?;
    check!(matches!(outcome, RosterVerdict::Quarantined {
        reason: RosterQuarantine::NoReadableRows, rejected
    } if rejected.is_empty()));
    Ok(())
}
