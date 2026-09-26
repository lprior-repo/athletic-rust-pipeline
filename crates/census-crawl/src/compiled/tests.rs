use super::header::header;
use super::*;
use census_domain::model::CentiSeconds;
use census_domain::model::{Gender, Mark, SourceRef};

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

/// Shape of the 2026 regional exports: two event blocks per page, relay on the left, a
/// preliminary heat on the right that carries a qualifier letter instead of points.
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
fn regional_export_parses_both_blocks_of_a_page() {
    let lines = crate::hytek::lines_from_pdf_text(REGIONAL);
    let meet = parse(&lines, source(), 2026).expect("the compiled export has a meet header");
    assert_eq!(meet.name, "D1 Regional 8B - Appleton North");
    assert_eq!(meet.date, "2026-05-26");
    assert_eq!(
        meet.events.len(),
        2,
        "one event per block: {:?}",
        meet.events
    );

    let relay = &meet.events[0];
    assert_eq!(relay.label, "4x800 Relay");
    assert_eq!(relay.gender, Gender::Girls);
    assert_eq!(relay.division.as_deref(), Some("Division 1"));
    assert_eq!(relay.round.as_deref(), Some("finals"));
    let winner = &relay.rows[0];
    assert_eq!(winner.school, "HORTONVILLE");
    assert_eq!(winner.mark, Mark::TimeSeconds(CentiSeconds::new(59511)));
    assert_eq!(winner.points, Some(10.0));
    assert_eq!(
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
    assert_eq!(
        relay.rows.len(),
        3,
        "every relay team on the page is read: {:?}",
        relay.rows
    );
    assert_eq!(relay.rows[1].school, "APPLETON NORTH");
    assert_eq!(
        relay.rows[2].mark,
        Mark::TimeSeconds(CentiSeconds::new(60338))
    );
    assert_eq!(
        relay.rows[2].legs,
        Vec::new(),
        "a team whose legs are printed outside the excerpt keeps no invented legs"
    );

    let dash = &meet.events[1];
    assert_eq!(dash.label, "100 Meters");
    assert_eq!(dash.round.as_deref(), Some("preliminaries"));
    assert_eq!(
        dash.rows.len(),
        8,
        "every prelim row is read: {:?}",
        dash.rows
    );
    let leader = &dash.rows[0];
    assert_eq!(leader.name, "Parrish, Ashley");
    assert_eq!(leader.grade, census_domain::model::Grade::new(11));
    assert_eq!(leader.place, Some(1));
    assert_eq!(leader.school, "APPLETON NOR\u{2026}");
    assert_eq!(leader.mark, Mark::TimeSeconds(CentiSeconds::new(1230)));
    assert_eq!(leader.points, None);
}

#[test]
fn print_artifacts_do_not_become_part_of_the_meet_name() {
    let lines = crate::hytek::lines_from_pdf_text(
            "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n",
        );
    let (name, date) = header(&lines).expect("the page stamp still carries the name");
    assert_eq!(name, "D3 Regional 4B - Deerfield");
    assert_eq!(date.as_deref(), Some("2025-05-27"));
}

#[test]
fn a_file_without_a_header_is_not_claimed() {
    let lines = vec!["Girls' 100 Meters Division 1   Finals".to_string()];
    assert!(parse(&lines, source(), 2026).is_none());
}
