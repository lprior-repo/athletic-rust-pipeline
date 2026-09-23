//! Row accounting: the published counters, the block a row belongs to, and the legs a relay row owns.
//!
//! The compiled page prints two events side by side and reads each one out of its own slice of the
//! line. A row read against the neighbouring block would publish an athlete under the wrong event, and
//! a leg line attached to the wrong team would put a runner on a relay they never ran.

use super::{parse_body, LAYOUTS, REGIONAL};

#[test]
fn the_counters_cover_the_rows_the_events_carry() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|| panic!("{name}: the body is a meet"));
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        assert!(
            meet.rows_parsed >= rows,
            "{name}: every published row is counted (parsed={} rows={rows})",
            meet.rows_parsed
        );
        assert!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
        if !meet.events.iter().any(|event| event.kind.is_relay()) {
            assert_eq!(
                meet.rows_parsed, rows,
                "{name}: with no leg line on the page the counter is exactly the rows"
            );
            assert_eq!(
                meet.rows_skipped, 0,
                "{name}: every data row of a readable grid is a performance"
            );
        }
    }
}

/// Grades are why track feeds the class-of-2027 census: the heat prints a `Yr` for every athlete and
/// the relay block prints one per leg, and a page that stopped publishing them would fail here
/// instead of silently gutting the cohort.
#[test]
fn every_row_and_leg_of_these_pages_carries_a_year() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|| panic!("{name}: the body is a meet"));
        for event in &meet.events {
            for row in &event.rows {
                if event.kind.is_relay() {
                    assert!(
                        row.legs.iter().all(|leg| leg.grade.is_some()),
                        "{name}: a printed relay leg carries its runner's year"
                    );
                } else {
                    assert!(
                        row.grade.is_some(),
                        "{name}: {} carries the athlete's year",
                        row.name
                    );
                }
            }
        }
    }
}

/// The two blocks of a page are read against their own columns: a relay row names a team and its
/// legs, an individual row names an athlete and never carries legs.
#[test]
fn a_blocks_rows_belong_to_that_blocks_event() {
    let meet = parse_body(REGIONAL).expect("the two-block page is a meet");

    let relay = meet
        .events
        .iter()
        .find(|event| event.kind.is_relay())
        .expect("the page prints a relay");
    assert!(
        relay.rows.iter().all(|row| row.name.is_empty()),
        "a relay row names its team, never an athlete: {:?}",
        relay.rows
    );
    assert!(
        relay.rows.iter().all(|row| !row.school.is_empty()),
        "a relay row names the team that ran it"
    );
    assert_eq!(
        relay.rows.len(),
        3,
        "the page's three teams are read, and no heat athlete is read as a team"
    );

    let heat = meet
        .events
        .iter()
        .find(|event| !event.kind.is_relay())
        .expect("the page prints a heat");
    assert_eq!(heat.rows.len(), 8, "the heat's eight rows are read");
    assert!(
        heat.rows.iter().all(|row| !row.name.is_empty()),
        "an individual row names its athlete"
    );
    assert!(
        heat.rows.iter().all(|row| row.legs.is_empty()),
        "legs belong to relay rows alone"
    );
    assert!(
        heat.rows.iter().all(|row| row.points.is_none()),
        "a preliminary heat awards no points: {:?}",
        heat.rows
    );
}

/// The legs printed beneath a relay row are that row's: they are numbered in the order the page
/// printed them, and a team whose legs fell outside the page keeps none.
#[test]
fn a_relay_rows_legs_are_the_legs_printed_beneath_it() {
    let meet = parse_body(REGIONAL).expect("the two-block page is a meet");
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind.is_relay())
        .expect("the page prints a relay");

    let legs_of = |index: usize| -> Vec<&str> {
        relay.rows[index]
            .legs
            .iter()
            .map(|leg| leg.name.as_str())
            .collect()
    };
    assert_eq!(
        legs_of(0),
        [
            "Wloszczynski, Lexi",
            "Young, Ellie",
            "Falbo, Hailey",
            "Huza, Hannah"
        ],
        "both leg lines belong to the team printed above them"
    );
    assert_eq!(
        legs_of(1),
        [
            "Dehlinger, Audry",
            "Brazzale, Elise",
            "Busch, Sophia",
            "Helmbrecht, Ava"
        ],
        "the second team's legs never join the first team's row"
    );
    let positions: Vec<u8> = relay.rows[0].legs.iter().map(|leg| leg.position).collect();
    assert_eq!(
        positions,
        [1, 2, 3, 4],
        "the legs keep the order the page printed them in"
    );
    assert!(
        relay.rows[2].legs.is_empty(),
        "a team whose legs are printed outside the excerpt keeps no invented legs"
    );
}
