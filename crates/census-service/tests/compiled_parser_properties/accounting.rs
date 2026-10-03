use super::{parse_body, LAYOUTS, REGIONAL};

#[test]
fn the_counters_cover_the_rows_the_events_carry() -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).ok_or_else(|| format!("{name}: body is not a meet"))?;
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        check!(
            meet.rows_parsed >= rows,
            "{name}: every published row is counted (parsed={} rows={rows})",
            meet.rows_parsed
        );
        check!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
        if !meet.events.iter().any(|event| event.kind.is_relay()) {
            check!(eq; meet.rows_parsed, rows,
            "{name}: with no leg line on the page the counter is exactly the rows");
            check!(eq; meet.rows_skipped, 0,
            "{name}: every data row of a readable grid is a performance");
        }
    }
    Ok(())
}

#[test]
fn every_row_and_leg_of_these_pages_carries_a_year() -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).ok_or_else(|| format!("{name}: body is not a meet"))?;
        for event in &meet.events {
            for row in &event.rows {
                if event.kind.is_relay() {
                    check!(
                        row.legs.iter().all(|leg| leg.grade.is_some()),
                        "{name}: a printed relay leg carries its runner's year"
                    );
                } else {
                    check!(
                        row.grade.is_some(),
                        "{name}: {} carries the athlete's year",
                        row.name
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_blocks_rows_belong_to_that_blocks_event() -> Result<(), Box<dyn std::error::Error>> {
    let meet = parse_body(REGIONAL).ok_or("the two-block page is not a meet")?;

    let relay = meet
        .events
        .iter()
        .find(|event| event.kind.is_relay())
        .ok_or("the page prints no relay")?;
    check!(
        relay.rows.iter().all(|row| row.name.is_empty()),
        "a relay row names its team, never an athlete: {:?}",
        relay.rows
    );
    check!(
        relay.rows.iter().all(|row| !row.school.is_empty()),
        "a relay row names the team that ran it"
    );
    check!(eq; relay.rows.len(),
    3,
    "the page's three teams are read, and no heat athlete is read as a team");

    let heat = meet
        .events
        .iter()
        .find(|event| !event.kind.is_relay())
        .ok_or("the page prints no heat")?;
    check!(eq; heat.rows.len(), 8, "the heat's eight rows are read");
    check!(
        heat.rows.iter().all(|row| !row.name.is_empty()),
        "an individual row names its athlete"
    );
    check!(
        heat.rows.iter().all(|row| row.legs.is_empty()),
        "legs belong to relay rows alone"
    );
    check!(
        heat.rows.iter().all(|row| row.points.is_none()),
        "a preliminary heat awards no points: {:?}",
        heat.rows
    );
    Ok(())
}

#[test]
fn a_relay_rows_legs_are_the_legs_printed_beneath_it() -> Result<(), Box<dyn std::error::Error>> {
    let meet = parse_body(REGIONAL).ok_or("the two-block page is not a meet")?;
    let relay = meet
        .events
        .iter()
        .find(|event| event.kind.is_relay())
        .ok_or("the page prints no relay")?;

    let legs_of = |index: usize| -> Vec<&str> {
        relay.rows[index]
            .legs
            .iter()
            .map(|leg| leg.name.as_str())
            .collect()
    };
    check!(eq; legs_of(0),
    [
        "Wloszczynski, Lexi",
        "Young, Ellie",
        "Falbo, Hailey",
        "Huza, Hannah"
    ],
    "both leg lines belong to the team printed above them");
    check!(eq; legs_of(1),
    [
        "Dehlinger, Audry",
        "Brazzale, Elise",
        "Busch, Sophia",
        "Helmbrecht, Ava"
    ],
    "the second team's legs never join the first team's row");
    let positions: Vec<u8> = relay.rows[0].legs.iter().map(|leg| leg.position).collect();
    check!(eq; positions,
    [1, 2, 3, 4],
    "the legs keep the order the page printed them in");
    check!(
        relay.rows[2].legs.is_empty(),
        "a team whose legs are printed outside the excerpt keeps no invented legs"
    );
    Ok(())
}
