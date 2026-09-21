use super::*;

/// Verbatim `<h3>` + finish-list table from
/// `https://www.wiaawi.org/Portals/0/PDF/Results/Cross_Country/2023/racinesectionalb.htm`
/// (WIAA Division 2 Racine sectional, boys race, 2023).
const FINISH_LIST: &str =
    include_str!("../../../tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

#[test]
fn finish_list_rows_carry_place_grade_school_and_the_final_time() -> CrawlResult<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    assert_eq!(meet.name, "WIAA D2 XC Sectionals - Boys Race");
    assert_eq!(
        meet.date, "2023",
        "RaceDay publishes no date; year precision is explicit"
    );
    let event = &meet.events[0];
    assert_eq!(event.kind, EventKind::CrossCountry);
    assert_eq!(event.gender, Gender::Boys);
    assert_eq!(event.division.as_deref(), Some("Division 2"));
    let winner = &event.rows[0];
    assert_eq!(winner.name, "Jack Hefty");
    assert_eq!(winner.grade.map(Grade::get), Some(11));
    assert_eq!(winner.school, "Whitewater");
    assert_eq!(
        winner.mark,
        Mark::TimeSeconds(1033.69),
        "17:13.69 is the finish, not a mile split"
    );
    assert!(event.rows.len() > 20, "got {} rows", event.rows.len());
    Ok(())
}

#[test]
fn the_team_summary_table_never_becomes_athlete_rows() -> CrawlResult<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    for row in &meet.events[0].rows {
        assert!(
            row.grade.is_some(),
            "a team summary row has no grade: {row:?}"
        );
    }
    Ok(())
}

#[test]
fn the_published_row_counters_are_the_rows_the_reader_took() -> CrawlResult<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    assert_eq!(
        meet.rows_parsed, rows,
        "the counter agrees with the rows the meet carries"
    );
    assert_eq!(rows, 82, "the finish list publishes 82 athletes");
    assert_eq!(
        meet.rows_skipped, 0,
        "every data row of the grid carries an athlete and a finish"
    );
    Ok(())
}

/// A one-table export whose second row carries no athlete name: a grid row the layout cannot read
/// as a performance, beside one it can.
const DECLINED_ROW: &str = r#"
<h3>WIAA D3 XC Sectionals - Girls Race Team Finish List-XC</h3>
<table class="data-display">
<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Finish</th></tr></thead>
<tbody>
<tr><td>1</td><td>Ada Bell</td><td>11</td><td>Whitewater</td><td>19:02.10</td></tr>
<tr><td>2</td><td></td><td>12</td><td>East Troy</td><td>20:11.44</td></tr>
</tbody>
</table>
"#;

#[test]
fn a_declined_grid_row_is_reported_as_skipped_not_counted_as_parsed() -> CrawlResult<()> {
    let meet = parse(DECLINED_ROW, source(), 2023)?;
    assert_eq!(meet.rows_parsed, 1, "only the row with an athlete is a row");
    assert_eq!(
        meet.rows_skipped, 1,
        "the row without an athlete is reported, never guessed"
    );
    assert_eq!(meet.events[0].rows[0].name, "Ada Bell");
    Ok(())
}

#[test]
fn divisions_parse_from_both_spellings() {
    assert_eq!(
        division_of("WIAA D1 XC Sectionals"),
        Some("Division 1".to_string())
    );
    assert_eq!(
        division_of("Division 3 Boys Results"),
        Some("Division 3".to_string())
    );
    assert_eq!(division_of("Boys Race"), None);
}
