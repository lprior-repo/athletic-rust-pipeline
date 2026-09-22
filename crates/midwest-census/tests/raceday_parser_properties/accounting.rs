//! Row accounting: the published counters, and what a row has to carry to be published at all.
//!
//! The census grades class-of-2027 runners from these rows, so a row that is dropped changes a cohort
//! and a row published without an athlete or a finish is a performance nobody ran. Both counters are
//! asserted on the committed capture and on a grid with a row the layout cannot read.

use super::{parse_body, rendered_rows, LAYOUTS};
use census_domain::model::Mark;

#[test]
fn the_counters_are_the_rows_the_events_carry() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        assert_eq!(
            meet.rows_parsed, rows,
            "{name}: the published counter is the rows the meet carries"
        );
        assert_eq!(
            meet.rows_parsed,
            rendered_rows(&meet).len(),
            "{name}: and the rows are the ones the events hold"
        );
        assert!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
    }
}

/// Every published row carries the athlete, the season's runner's year, the school and a finish —
/// the four facts the census reads out of this layout.
#[test]
fn every_published_row_names_an_athlete_with_a_year_and_a_finish() {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        for row in meet.events.iter().flat_map(|event| event.rows.iter()) {
            assert!(
                !row.name.trim().is_empty(),
                "{name}: a row without an athlete is reported as skipped, never published: {row:?}"
            );
            assert!(
                !row.school.trim().is_empty(),
                "{name}: a row keeps the school it ran for: {row:?}"
            );
            assert!(
                row.grade.is_some(),
                "{name}: the grid publishes the runner's year, which is why cross-country feeds the \
                 class-of-2027 census: {row:?}"
            );
            assert!(
                matches!(row.mark, Mark::TimeSeconds(_)),
                "{name}: the finish column is a time, not a split: {row:?}"
            );
        }
    }
}

/// The committed capture publishes a team summary beneath the finish list; its rows carry no runner
/// and no year, so they are never published as performances — the whole grid is not the whole file.
#[test]
fn the_team_summary_rows_never_become_performances() {
    let meet = parse_body(super::CAPTURE).expect("the capture is a meet");
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    assert_eq!(rows, 82, "the finish list publishes 82 athletes");
    assert_eq!(
        meet.rows_skipped, 0,
        "every data row of the finish grid carries an athlete and a finish, so none is declined"
    );
    assert!(
        meet.events
            .iter()
            .flat_map(|event| event.rows.iter())
            .all(|row| row.place.is_some()),
        "a summary row has no place, and no summary row is among the published rows"
    );
}

/// A row the layout cannot read as a performance is reported, not guessed and not silently dropped:
/// the report's skip count is the only place it surfaces.
#[test]
fn a_nameless_grid_row_is_reported_as_skipped() {
    let meet = parse_body(super::DECLINED_ROW).expect("the page is a meet");
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    assert_eq!(rows, 1, "only the row with an athlete is a performance");
    assert_eq!(meet.rows_parsed, 1, "and it is the one counted");
    assert_eq!(
        meet.rows_skipped, 1,
        "the nameless row is reported as skipped rather than published or dropped in silence"
    );
}
