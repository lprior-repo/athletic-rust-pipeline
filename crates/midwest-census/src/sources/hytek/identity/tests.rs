//! The identity reader against the artifact that needs it: a WIAA release whose capture drops the
//! jump section's own header and leaves its individual rows under the relay header above them.

use super::individual_identity;
use crate::sources::hytek::{lines_from_html, parse};
use crate::sources::result_file::ParsedRow;
use census_domain::model::{EventKind, Grade, Mark, SourceRef};

/// Verbatim slice of
/// `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm` — the same
/// capture the Hy-Tek reader's own tests read.
const SECTIONS: &str =
    include_str!("../../../../tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");

#[test]
fn an_individual_row_under_a_relay_header_keeps_its_athlete_grade_and_school() {
    // The capture drops the jump section's own header, so the published row
    // `-- Donavin Bond              10 Nicolet                   FOUL        2`
    // sits under the relay header that names only a `School` column. It is still an individual
    // row: reading its label as a school would file the athlete under the school and empty the
    // name, which downstream reports as an unresolved school instead of a performance.
    let meet = parse(&lines_from_html(SECTIONS), SourceRef::new("wiaa_results", None))
        .expect("the fixture has a meet header");
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .expect("the fixture publishes the 4x100 relay the row is printed under");
    let fouled = relay
        .rows
        .iter()
        .find(|row| row.name == "Donavin Bond")
        .unwrap_or_else(|| panic!("the FOUL row keeps its athlete: {:?}", relay.rows));
    assert_eq!(
        fouled,
        &ParsedRow {
            place: None,
            name: "Donavin Bond".to_string(),
            grade: Grade::new(10),
            school: "Nicolet".to_string(),
            mark: Mark::Raw("FOUL".to_string()),
            wind_mps: None,
            heat: Some("2".to_string()),
            points: None,
            legs: Vec::new(),
        },
        "the row reads as the individual row it is, with the published no-mark"
    );
}

#[test]
fn the_relay_rows_around_it_stay_school_labels() {
    // The same section's relay rows name schools and list their legs; the identity reader must not
    // reach into them.
    let meet = parse(&lines_from_html(SECTIONS), SourceRef::new("wiaa_results", None))
        .expect("the fixture has a meet header");
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .expect("the fixture publishes the 4x100 relay");
    for row in &relay.rows {
        assert!(
            !row.school.chars().any(|ch| ch.is_ascii_digit()),
            "no school label carries a grade or a mark: {row:?}"
        );
        if !row.legs.is_empty() {
            assert!(row.name.is_empty(), "a relay row names a school: {row:?}");
        }
    }
    assert_eq!(
        relay
            .rows
            .iter()
            .filter(|row| row.school == "Homestead")
            .count(),
        1
    );
}

#[test]
fn the_individual_shape_reads_back_as_athlete_grade_and_school() {
    assert_eq!(
        individual_identity("Donavin Bond 10 Nicolet"),
        Some((
            "Donavin Bond".to_string(),
            Grade::new(10).expect("10 is a grade"),
            "Nicolet".to_string()
        ))
    );
}

#[test]
fn a_school_label_that_is_not_an_identity_is_left_alone() {
    // The shape is exact, because guessing an identity from a school label is worse than leaving
    // it: two grade-shaped tokens, a grade with no run on one side, or digits in the tail are all
    // declared unknown rather than split.
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
