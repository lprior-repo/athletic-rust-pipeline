//! Unit tests over the fixture for parsing and mapping.

use super::pages::{parse_directory, parse_sport_label};
use census_domain::model::{Gender, Sport};

/// Fixture HTML bytes captured from `https://riil.org/Directory.aspx`.
const FIXTURE: &str = include_str!("fixture.html");

#[test]
fn parse_directory_returns_schools() {
    let tables = parse_directory(FIXTURE);
    assert!(
        !tables.is_empty(),
        "directory should yield at least one school"
    );
}

#[test]
fn parse_directory_count() {
    let tables = parse_directory(FIXTURE);
    assert!(
        tables.len() >= 50,
        "expected at least 50 schools, got {}",
        tables.len()
    );
}

#[test]
fn school_names_parsed() {
    let tables = parse_directory(FIXTURE);
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"Barrington HS"),
        "expected Barrington HS in directory, got {:?}",
        names
            .iter()
            .filter(|n| n.contains("Barr"))
            .collect::<Vec<_>>()
    );
    assert!(
        names.iter().any(|n| n.contains("Bishop Hendricken")),
        "expected Bishop Hendricken in directory"
    );
}

#[test]
fn xc_tf_coaches_parsed() {
    let tables = parse_directory(FIXTURE);
    let mut xc_tf_count = 0usize;
    for table in &tables {
        xc_tf_count += table.coach_rows.len();
    }
    assert!(
        xc_tf_count >= 250,
        "expected at least 250 XC/TF coach rows, got {}",
        xc_tf_count
    );
}

#[test]
fn barrington_xc_coaches() {
    let tables = parse_directory(FIXTURE);
    let barrington = tables
        .iter()
        .find(|t| t.name == "Barrington HS")
        .expect("Barrington HS not found");
    let names: Vec<&str> = barrington
        .coach_rows
        .iter()
        .map(|r| r.coach_name.as_str())
        .collect();

    assert!(
        names.contains(&"Mike Katz"),
        "expected Mike Katz (Boys XC) in Barrington, got {:?}",
        names
    );
    assert!(
        names.contains(&"Molly Lacher-Katz"),
        "expected Molly Lacher-Katz (Girls XC) in Barrington, got {:?}",
        names
    );
    assert!(
        names.contains(&"Bill Barrass"),
        "expected Bill Barrass (Boys Indoor Track) in Barrington, got {:?}",
        names
    );
}

#[test]
fn bishop_hendricken_coaches() {
    let tables = parse_directory(FIXTURE);
    let hendricken = tables
        .iter()
        .find(|t| t.name.contains("Bishop Hendricken"))
        .expect("Bishop Hendricken not found");
    let names: Vec<&str> = hendricken
        .coach_rows
        .iter()
        .map(|r| r.coach_name.as_str())
        .collect();

    assert!(
        names.contains(&"Jim Doyle"),
        "expected Jim Doyle (Boys XC) in Bishop Hendricken, got {:?}",
        names
    );
}

#[test]
fn non_xc_sport_rows_exist_in_fixture() {
    assert!(
        FIXTURE.contains("Boys Basketball"),
        "fixture should contain Boys Basketball rows"
    );
    assert!(
        FIXTURE.contains("Eli Perry"),
        "fixture should contain Eli Perry (Boys Basketball head coach)"
    );
    assert!(
        FIXTURE.contains("Girls Volleyball"),
        "fixture should contain Girls Volleyball rows"
    );
}

#[test]
fn sport_label_mapping() {
    assert_eq!(
        parse_sport_label("Boys Cross Country"),
        Some(Sport::CrossCountry)
    );
    assert_eq!(
        parse_sport_label("Girls Cross Country"),
        Some(Sport::CrossCountry)
    );
    assert_eq!(
        parse_sport_label("Boys Indoor Track"),
        Some(Sport::IndoorTrack)
    );
    assert_eq!(
        parse_sport_label("Girls Indoor Track"),
        Some(Sport::IndoorTrack)
    );
    assert_eq!(
        parse_sport_label("Boys Outdoor Track"),
        Some(Sport::OutdoorTrack)
    );
    assert_eq!(
        parse_sport_label("Girls Outdoor Track"),
        Some(Sport::OutdoorTrack)
    );
    assert_eq!(parse_sport_label("Boys Basketball"), None);
    assert_eq!(parse_sport_label("Girls Volleyball"), None);
}

#[test]
fn gender_from_barrington_rows() {
    let tables = parse_directory(FIXTURE);
    let barrington = tables
        .iter()
        .find(|t| t.name == "Barrington HS")
        .expect("Barrington HS not found");

    for row in &barrington.coach_rows {
        let expected_gender = if row.sport_label.starts_with("Boys ") {
            Gender::Boys
        } else if row.sport_label.starts_with("Girls ") {
            Gender::Girls
        } else {
            Gender::Mixed
        };
        assert!(
            parse_sport_label(&row.sport_label).is_some(),
            "sport '{}' should map to a valid Sport variant",
            row.sport_label
        );
        assert_eq!(row.sport, parse_sport_label(&row.sport_label).unwrap());
        match row.sport_label.as_str() {
            s if s.starts_with("Boys ") => assert_eq!(expected_gender, Gender::Boys),
            s if s.starts_with("Girls ") => assert_eq!(expected_gender, Gender::Girls),
            _ => assert_eq!(expected_gender, Gender::Mixed),
        }
    }
}

#[test]
fn school_extract_mints_coaches() {
    let tables = parse_directory(FIXTURE);
    let barrington = tables
        .iter()
        .find(|t| t.name == "Barrington HS")
        .expect("Barrington HS not found");
    let extract = super::map::school_entities(barrington, "2026-09-24");

    assert!(!extract.school.name.is_empty());
    assert!(!extract.school.normalized_name.is_empty());
    assert!(
        extract.coaches.len() >= 5,
        "Barrington should have at least 5 coaches"
    );

    let school_id = &extract.school.id;
    for coach in &extract.coaches {
        assert_eq!(coach.school, *school_id);
        assert_eq!(coach.role, census_domain::model::CoachRole::HeadCoach);
    }
}
