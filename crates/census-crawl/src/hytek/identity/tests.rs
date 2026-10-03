use super::individual_identity;
use crate::hytek::{lines_from_html, parse};
use crate::result_file::ParsedRow;
use census_domain::model::{EventKind, Grade, Mark, SourceRef};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SECTIONS: &str =
    include_str!("../../../tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");

#[test]
fn an_individual_row_under_a_relay_header_keeps_its_athlete_grade_and_school() -> TestResult {
    let meet = parse(
        &lines_from_html(SECTIONS),
        SourceRef::new("wiaa_results", None),
    )
    .ok_or("the fixture has a meet header")?;
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .ok_or("the fixture publishes the 4x100 relay the row is printed under")?;
    let fouled = relay
        .rows
        .iter()
        .find(|row| row.name == "Donavin Bond")
        .ok_or("the FOUL row keeps its athlete")?;
    check!(eq;
        fouled,
        &ParsedRow {
            place: None,
            name: "Donavin Bond".to_string(),
            grade: Grade::new(10),
            school: "Nicolet".to_string(),
            mark: Mark::Raw("FOUL".to_string()),
            timing: None,
            wind_mps: None,
            heat: Some("2".to_string()),
            points: None,
            legs: Vec::new(),
        },
        "the row reads as the individual row it is, with the published no-mark"
    );
    Ok(())
}

#[test]
fn the_relay_rows_around_it_stay_school_labels() -> TestResult {
    let meet = parse(
        &lines_from_html(SECTIONS),
        SourceRef::new("wiaa_results", None),
    )
    .ok_or("the fixture has a meet header")?;
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .ok_or("the fixture publishes the 4x100 relay")?;
    for row in &relay.rows {
        check!(
            !row.school.chars().any(|ch| ch.is_ascii_digit()),
            "no school label carries a grade or a mark: {row:?}"
        );
        if !row.legs.is_empty() {
            check!(row.name.is_empty(), "a relay row names a school: {row:?}");
        }
    }
    check!(eq;
        relay
            .rows
            .iter()
            .filter(|row| row.school == "Homestead")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn the_individual_shape_reads_back_as_athlete_grade_and_school() -> TestResult {
    check!(eq;
        individual_identity("Donavin Bond 10 Nicolet"),
        Some((
            "Donavin Bond".to_string(),
            Grade::new(10).ok_or("10 is a grade")?,
            "Nicolet".to_string()
        ))
    );
    Ok(())
}

#[test]
fn a_school_label_that_is_not_an_identity_is_left_alone() {
    assert_eq!(individual_identity("Nicolet"), None);
    assert_eq!(individual_identity("Wisconsin Luth."), None);
    assert_eq!(individual_identity("10 Nicolet"), None, "a grade lead");
    assert_eq!(individual_identity("Nicolet 10"), None, "a grade tail");
    assert_eq!(
        individual_identity("Donavin Bond 10 11 Nicolet"),
        None,
        "two grade-shaped tokens have no single year"
    );
    assert_eq!(
        individual_identity("Donavin Bond 10 40-08.50"),
        None,
        "a mark left in the column is not a school"
    );
    assert_eq!(
        individual_identity("25 Nathan Tranberg 10 Wisconsin Luth."),
        None,
        "a leading place keeps the label a label"
    );
}
