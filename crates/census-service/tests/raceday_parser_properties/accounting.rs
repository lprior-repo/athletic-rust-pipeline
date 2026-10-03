use super::{parse_body, rendered_rows, LAYOUTS};
use census_domain::model::Mark;

#[test]
fn the_counters_are_the_rows_the_events_carry() -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body)?;
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        check!(eq; meet.rows_parsed, rows,
        "{name}: the published counter is the rows the meet carries");
        check!(eq; meet.rows_parsed,
        rendered_rows(&meet).len(),
        "{name}: and the rows are the ones the events hold");
        check!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
    }
    Ok(())
}

#[test]
fn every_published_row_names_an_athlete_with_a_year_and_a_finish(
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body)?;
        for row in meet.events.iter().flat_map(|event| event.rows.iter()) {
            check!(
                !row.name.trim().is_empty(),
                "{name}: a row without an athlete is reported as skipped, never published: {row:?}"
            );
            check!(
                !row.school.trim().is_empty(),
                "{name}: a row keeps the school it ran for: {row:?}"
            );
            check!(row.grade.is_some(),
            "{name}: the grid publishes the runner's year, which is why cross-country feeds the \
             class-of-2027 census: {row:?}");
            check!(
                matches!(row.mark, Mark::TimeSeconds(_)),
                "{name}: the finish column is a time, not a split: {row:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_team_summary_rows_never_become_performances() -> Result<(), Box<dyn std::error::Error>> {
    let meet = parse_body(super::CAPTURE)?;
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    check!(eq; rows, 82, "the finish list publishes 82 athletes");
    check!(eq; meet.rows_skipped, 0,
    "every data row of the finish grid carries an athlete and a finish, so none is declined");
    check!(
        meet.events
            .iter()
            .flat_map(|event| event.rows.iter())
            .all(|row| row.place.is_some()),
        "a summary row has no place, and no summary row is among the published rows"
    );
    Ok(())
}

#[test]
fn a_nameless_grid_row_is_reported_as_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let meet = parse_body(super::DECLINED_ROW)?;
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    check!(eq; rows, 1, "only the row with an athlete is a performance");
    check!(eq; meet.rows_parsed, 1, "and it is the one counted");
    check!(eq; meet.rows_skipped, 1,
    "the nameless row is reported as skipped rather than published or dropped in silence");
    Ok(())
}
