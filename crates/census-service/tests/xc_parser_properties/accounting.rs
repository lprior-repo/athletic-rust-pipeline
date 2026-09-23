//! Row accounting: every row a file carries is counted, and it lands in the event of its own
//! section.
//!
//! The census grades class-of-2027 runners from these rows, so a row that is dropped changes a
//! cohort and a row that lands in the wrong event grades the wrong race. Both are asserted on the
//! three published layouts and on a page where a race is replayed after another race.

use super::{parse_body, rows_of, LAYOUTS, REPLAYED};
use census_domain::model::Gender;

#[test]
fn every_row_is_counted_and_every_event_carries_one() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|| panic!("{name}: the body is a meet"));
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        assert_eq!(
            meet.rows_parsed, rows,
            "{name}: the published counter equals the rows the events hold"
        );
        assert!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
    }
}

/// Grades are why cross-country feeds the class-of-2027 census rather than only a results table:
/// every row of every layout carries one, and a layout that stopped publishing them would fail here
/// instead of silently gutting the cohort.
#[test]
fn every_row_of_these_layouts_carries_a_grade() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|| panic!("{name}: the body is a meet"));
        for event in &meet.events {
            for row in &event.rows {
                assert!(
                    row.grade.is_some(),
                    "{name}: {} carries the runner's grade",
                    row.name
                );
            }
        }
    }
}

/// The layouts are not one race per page: a walk re-anchors on each banner, so a race that is
/// resumed after another race must keep its own rows. A row that lands in the section that happened
/// to be open last grades the wrong race.
#[test]
fn a_replayed_section_keeps_its_own_rows() {
    let meet = parse_body(REPLAYED).expect("the replayed body is a meet");

    let boys: Vec<&str> = rows_of(&meet, Gender::Boys)
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    let girls: Vec<&str> = rows_of(&meet, Gender::Girls)
        .iter()
        .map(|row| row.name.as_str())
        .collect();

    assert_eq!(
        boys,
        ["Aaron Alpha", "Bram Beta", "Ethan Epsilon", "Finn Zeta"],
        "both boys blocks belong to the boys race, in file order"
    );
    assert_eq!(
        girls,
        ["Cara Gamma", "Dana Delta"],
        "the girls block belongs to the girls race alone"
    );
}
