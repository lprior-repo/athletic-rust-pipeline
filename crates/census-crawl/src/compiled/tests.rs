use super::header::header;
use super::*;
use census_domain::model::ExactSeconds;
use census_domain::model::{Gender, Mark, SourceRef};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

const REGIONAL: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                        Appleton North HS  Tue, May 26, 2026
                                                                     Results
Girls' 4x800 Relay Division 1                     Finals                    Girls' 100 Meters Division 1              Prelims
       Team                    Relay        Finals             Pts               Athlete                 Yr Team              Prelims
1      HORTONVILLE             'A'          9:55.11            10           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q
    1) Wloszczynski, Lexi 10         2) Young, Ellie 9                      2    Thompson, Emily         12   APPLETON NOR…   12.74 q
    3) Falbo, Hailey 12              4) Huza, Hannah 12                     3    Rades, Jayla            11   HORTONVILLE     12.83 Q
                                                                            4    Rezash, Johannah        12   WEST DE PERE    13.12 q
2      APPLETON NORTH          'A'          9:56.50            8
                                                                            5    Lopez, Eilianyz         10   WEST DE PERE    13.38 q
    1) Dehlinger, Audry 11           2) Brazzale, Elise 10                  6    Hammen, Allie           9    APPLETON WEST   13.39 q
    3) Busch, Sophia 12              4) Helmbrecht, Ava 12
                                                                            7    Josephson, Sydney       9    KAUKAUNA        13.44 q
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12   APPLETON EAST   13.55 q
"#;

#[test]
fn regional_export_parses_both_blocks_of_a_page() -> TestResult {
    let lines = crate::hytek::lines_from_pdf_text(REGIONAL);
    let meet = parse(&lines, source(), 2026).ok_or("the compiled export has a meet header")?;
    check!(eq; meet.name, "D1 Regional 8B - Appleton North");
    check!(eq; meet.date, "2026-05-26");
    check!(eq;
        meet.events.len(),
        2,
        "one event per block: {:?}",
        meet.events
    );

    let relay = &meet.events[0];
    check!(eq; relay.label, "4x800 Relay");
    check!(eq; relay.gender, Gender::Girls);
    check!(eq; relay.division.as_deref(), Some("Division 1"));
    check!(eq; relay.round.as_deref(), Some("finals"));
    let winner = &relay.rows[0];
    check!(eq; winner.school, "HORTONVILLE");
    check!(eq; winner.mark, Mark::TimeSeconds(ExactSeconds::parse("595.11")?));
    check!(eq; winner.points, Some(10.0));
    check!(eq;
        winner.legs,
        vec![
            RelayLeg {
                position: 1,
                name: "Wloszczynski, Lexi".into(),
                grade: census_domain::model::Grade::new(10)
            },
            RelayLeg {
                position: 2,
                name: "Young, Ellie".into(),
                grade: census_domain::model::Grade::new(9)
            },
            RelayLeg {
                position: 3,
                name: "Falbo, Hailey".into(),
                grade: census_domain::model::Grade::new(12)
            },
            RelayLeg {
                position: 4,
                name: "Huza, Hannah".into(),
                grade: census_domain::model::Grade::new(12)
            },
        ],
        "both leg lines belong to the relay row above them"
    );
    check!(eq;
        relay.rows.len(),
        3,
        "every relay team on the page is read: {:?}",
        relay.rows
    );
    check!(eq; relay.rows[1].school, "APPLETON NORTH");
    check!(eq;
        relay.rows[2].mark,
        Mark::TimeSeconds(ExactSeconds::parse("603.38")?)
    );
    check!(eq;
        relay.rows[2].legs,
        Vec::new(),
        "a team whose legs are printed outside the excerpt keeps no invented legs"
    );

    let dash = &meet.events[1];
    check!(eq; dash.label, "100 Meters");
    check!(eq; dash.round.as_deref(), Some("preliminaries"));
    check!(eq;
        dash.rows.len(),
        8,
        "every prelim row is read: {:?}",
        dash.rows
    );
    let leader = &dash.rows[0];
    check!(eq; leader.name, "Parrish, Ashley");
    check!(eq; leader.grade, census_domain::model::Grade::new(11));
    check!(eq; leader.place, Some(1));
    check!(eq; leader.school, "APPLETON NOR\u{2026}");
    check!(eq; leader.mark, Mark::TimeSeconds(ExactSeconds::parse("12.30")?));
    check!(eq; leader.points, None);
    Ok(())
}

#[test]
fn print_artifacts_do_not_become_part_of_the_meet_name() -> TestResult {
    let lines = crate::hytek::lines_from_pdf_text(
            "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n",
        );
    let (name, date) = header(&lines).ok_or("the page stamp still carries the name")?;
    check!(eq; name, "D3 Regional 4B - Deerfield");
    check!(eq; date.as_deref(), Some("2025-05-27"));
    Ok(())
}

#[test]
fn a_file_without_a_header_is_not_claimed() {
    let lines = vec!["Girls' 100 Meters Division 1   Finals".to_string()];
    assert!(parse(&lines, source(), 2026).is_none());
}

#[test]
fn over_precision_timed_rows_do_not_fall_back_to_distance_or_hide_later_results() -> TestResult {
    let baseline = parse(&crate::hytek::lines_from_pdf_text(REGIONAL), source(), 2026)
        .ok_or("missing baseline compiled source meet")?;
    let body = REGIONAL.replace("12.30 Q", "12.3000000001 Q");
    let meet = parse(&crate::hytek::lines_from_pdf_text(&body), source(), 2026)
        .ok_or("missing compiled source meet")?;
    let dash = meet
        .events
        .iter()
        .find(|event| event.kind == census_domain::model::EventKind::Track100m)
        .ok_or("missing dash event")?;
    check!(eq; dash.rows.len(), 7);
    check!(eq; meet.rows_skipped, baseline.rows_skipped.checked_add(1).ok_or("rejected-row counter overflow")?);
    check!(dash.rows.iter().all(|row| row.name != "Parrish, Ashley"));
    let later = dash
        .rows
        .iter()
        .find(|row| row.name == "Thompson, Emily")
        .ok_or("missing later source row")?;
    check!(eq; later.mark, Mark::TimeSeconds(ExactSeconds::parse("12.74")?));
    Ok(())
}
