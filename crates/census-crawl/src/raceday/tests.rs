use super::*;

/// Verbatim `<h3>` + finish-list table from
/// `https://www.wiaawi.org/Portals/0/PDF/Results/Cross_Country/2023/racinesectionalb.htm`
/// (WIAA Division 2 Racine sectional, boys race, 2023).
const FINISH_LIST: &str =
    include_str!("../../tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

#[test]
fn finish_list_rows_carry_place_grade_school_and_the_final_time() -> anyhow::Result<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    anyhow::ensure!(
        meet.name == "WIAA D2 XC Sectionals - Boys Race",
        "left={:?} right={:?}",
        &meet.name,
        &"WIAA D2 XC Sectionals - Boys Race"
    );
    anyhow::ensure!(
        meet.date == "2023",
        "RaceDay publishes no date; year precision is explicit — left={:?} right={:?}",
        &meet.date,
        &"2023"
    );
    let event = &meet.events[0];
    anyhow::ensure!(
        event.kind == EventKind::CrossCountry,
        "left={:?} right={:?}",
        &event.kind,
        &EventKind::CrossCountry
    );
    anyhow::ensure!(
        event.gender == Gender::Boys,
        "left={:?} right={:?}",
        &event.gender,
        &Gender::Boys
    );
    {
        let left_value = &event.division.as_deref();
        let right_value = &(Some("Division 2"));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    let winner = &event.rows[0];
    anyhow::ensure!(
        winner.name == "Jack Hefty",
        "left={:?} right={:?}",
        &winner.name,
        &"Jack Hefty"
    );
    {
        let left_value = &(winner.grade.map(Grade::get));
        let right_value = &(Some(11));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    anyhow::ensure!(
        winner.school == "Whitewater",
        "left={:?} right={:?}",
        &winner.school,
        &"Whitewater"
    );
    {
        let left_value = &winner.mark;
        let right_value = &(Mark::TimeSeconds(CentiSeconds::new(103369)));
        anyhow::ensure!(
            left_value == right_value,
            "17:13.69 is the finish, not a mile split — left={left_value:?} right={right_value:?}"
        );
    }
    anyhow::ensure!(event.rows.len() > 20, "got {} rows", event.rows.len());
    Ok(())
}

#[test]
fn the_team_summary_table_never_becomes_athlete_rows() -> anyhow::Result<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    for row in &meet.events[0].rows {
        anyhow::ensure!(
            row.grade.is_some(),
            "a team summary row has no grade: {row:?}"
        );
    }
    Ok(())
}

#[test]
fn the_published_row_counters_are_the_rows_the_reader_took() -> anyhow::Result<()> {
    let meet = parse(FINISH_LIST, source(), 2023)?;
    let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
    anyhow::ensure!(
        meet.rows_parsed == rows,
        "the counter agrees with the rows the meet carries — left={:?} right={:?}",
        &meet.rows_parsed,
        &rows
    );
    anyhow::ensure!(
        rows == 82,
        "the finish list publishes 82 athletes — left={:?} right={:?}",
        &rows,
        &82
    );
    anyhow::ensure!(
        meet.rows_skipped == 0,
        "every data row of the grid carries an athlete and a finish — left={:?} right={:?}",
        &meet.rows_skipped,
        &0
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
fn a_declined_grid_row_is_reported_as_skipped_not_counted_as_parsed() -> anyhow::Result<()> {
    let meet = parse(DECLINED_ROW, source(), 2023)?;
    anyhow::ensure!(
        meet.rows_parsed == 1,
        "only the row with an athlete is a row — left={:?} right={:?}",
        &meet.rows_parsed,
        &1
    );
    anyhow::ensure!(
        meet.rows_skipped == 1,
        "the row without an athlete is reported, never guessed — left={:?} right={:?}",
        &meet.rows_skipped,
        &1
    );
    anyhow::ensure!(
        meet.events[0].rows[0].name == "Ada Bell",
        "left={:?} right={:?}",
        &meet.events[0].rows[0].name,
        &"Ada Bell"
    );
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

/// A one-table export where a team-member-place column carries numeric values (e.g. `5`) that
/// are also parseable as times. The finish-time column is identified by its header label, not by
/// position, so the parser yields the finish time, not the placement.
const PLACE_AS_TIME_FIXTURE: &str = r#"
<h3>Test XC Sectionals - Boys Race Team Finish List-XC</h3>
<table class="data-display">
<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Team Member Place</th><th>Finish</th></tr></thead>
<tbody>
<tr><td>1</td><td>Alice Runner</td><td>11</td><td>Whitewater</td><td>1</td><td>17:13.69</td></tr>
<tr><td>2</td><td>Bob Fast</td><td>12</td><td>East Troy</td><td>5</td><td>21:22.82</td></tr>
<tr><td>3</td><td>Charlie Slow</td><td>10</td><td>South Geneva</td><td>12</td><td>25:05.00</td></tr>
</tbody>
</table>
"#;

#[test]
fn a_placement_value_is_not_mistaken_for_a_finish_time() -> anyhow::Result<()> {
    let meet = parse(PLACE_AS_TIME_FIXTURE, source(), 2023)?;
    anyhow::ensure!(meet.events.len() == 1, "got {} events", meet.events.len());
    let event = &meet.events[0];
    anyhow::ensure!(event.rows.len() == 3, "got {} rows", event.rows.len());

    // Row 2: Team Member Place = 5, Finish = 21:22.82
    // The fix: identify the time column by label ("Finish"), not by position.
    // The old bug would have picked 5 (the right-most numeric cell) as 5 seconds.
    let bob = &event.rows[1];
    anyhow::ensure!(
        bob.name == "Bob Fast",
        "left={:?} right={:?}",
        &bob.name,
        &"Bob Fast"
    );
    let expected = Mark::TimeSeconds(CentiSeconds::new(128282));
    anyhow::ensure!(
        bob.mark == expected,
        "expected 21:22.82 = {:?}, got {:?} — the placement 5 was not mistaken for 5 seconds",
        expected,
        bob.mark
    );

    // Row 3: Team Member Place = 12, Finish = 25:05.00
    let charlie = &event.rows[2];
    anyhow::ensure!(
        charlie.name == "Charlie Slow",
        "left={:?} right={:?}",
        &charlie.name,
        &"Charlie Slow"
    );
    let expected = Mark::TimeSeconds(CentiSeconds::new(150500));
    anyhow::ensure!(
        charlie.mark == expected,
        "expected 25:05.00 = {:?}, got {:?}",
        expected,
        charlie.mark
    );

    // No rows should be skipped.
    anyhow::ensure!(
        meet.rows_skipped == 0,
        "left={:?} right={:?}",
        &meet.rows_skipped,
        &0
    );
    Ok(())
}

/// A multi-table export: one valid table and one table with no time column.
/// The valid table's rows are parsed; the no-time-column table's rows are rejected.
const MULTI_TABLE_NO_TIME_FIXTURE: &str = r#"
<h3>Test XC Sectionals - Boys Race Team Finish List-XC</h3>
<table class="data-display">
<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Finish</th></tr></thead>
<tbody>
<tr><td>1</td><td>Valid Athlete</td><td>11</td><td>Whitewater</td><td>18:30.00</td></tr>
</tbody>
</table>
<table class="data-display">
<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Team Member Place</th></tr></thead>
<tbody>
<tr><td>1</td><td>Missing Time 1</td><td>12</td><td>East Troy</td><td>5</td></tr>
<tr><td>2</td><td>Missing Time 2</td><td>10</td><td>South Geneva</td><td>12</td></tr>
</tbody>
</table>
"#;

#[test]
fn a_table_with_no_time_column_rejects_its_rows_with_a_reason() -> anyhow::Result<()> {
    let meet = parse(MULTI_TABLE_NO_TIME_FIXTURE, source(), 2023)?;
    // One valid table (1 row) and one no-time-column table (2 rejected rows).
    anyhow::ensure!(meet.events.len() == 1, "got {} events", meet.events.len());
    let event = &meet.events[0];
    anyhow::ensure!(
        event.rows.len() == 1,
        "expected 1 row, got {} — only the valid table contributes rows",
        event.rows.len()
    );
    anyhow::ensure!(
        event.rows[0].name == "Valid Athlete",
        "left={:?} right={:?}",
        &event.rows[0].name,
        &"Valid Athlete"
    );
    anyhow::ensure!(
        meet.rows_parsed == 1,
        "left={:?} right={:?}",
        &meet.rows_parsed,
        &1
    );
    anyhow::ensure!(
        meet.rows_skipped == 2,
        "left={:?} right={:?}",
        &meet.rows_skipped,
        &2
    );
    Ok(())
}
