use super::{parse_body, rows_of, LAYOUTS, REPLAYED};
use census_domain::model::Gender;

#[test]
fn every_row_is_counted_and_every_event_carries_one() -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).ok_or_else(|| format!("{name}: body is not a meet"))?;
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        check!(eq; meet.rows_parsed, rows,
        "{name}: the published counter equals the rows the events hold");
        check!(
            meet.events.iter().all(|event| !event.rows.is_empty()),
            "{name}: an event that received no row is dropped, never published empty"
        );
    }
    Ok(())
}

#[test]
fn every_row_of_these_layouts_carries_a_grade() -> Result<(), Box<dyn std::error::Error>> {
    for (name, body) in LAYOUTS {
        let meet = parse_body(body).ok_or_else(|| format!("{name}: body is not a meet"))?;
        for event in &meet.events {
            for row in &event.rows {
                check!(
                    row.grade.is_some(),
                    "{name}: {} carries the runner's grade",
                    row.name
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_replayed_section_keeps_its_own_rows() -> Result<(), Box<dyn std::error::Error>> {
    let meet = parse_body(REPLAYED).ok_or("replayed body did not parse")?;

    let boys: Vec<&str> = rows_of(&meet, Gender::Boys)
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    let girls: Vec<&str> = rows_of(&meet, Gender::Girls)
        .iter()
        .map(|row| row.name.as_str())
        .collect();

    check!(eq; boys,
    ["Aaron Alpha", "Bram Beta", "Ethan Epsilon", "Finn Zeta"],
    "both boys blocks belong to the boys race, in file order");
    check!(eq; girls,
    ["Cara Gamma", "Dana Delta"],
    "the girls block belongs to the girls race alone");
    Ok(())
}
