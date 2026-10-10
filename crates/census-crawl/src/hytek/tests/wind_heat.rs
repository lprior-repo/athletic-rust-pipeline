use super::*;
use census_domain::model::{EventKind, Mark};

const WIND_POINTS_HEADER: &str =
    "    Name                    Year School                  Finals  Wind Points";
const POINTS_ONLY_HEADER: &str =
    "    Name                    Year School                  Finals  Points";
const SEP: &str = "===========================================================================";

fn wind_meet(rows: &[String]) -> Result<ParsedMeet, String> {
    let mut lines = vec![
        "Licensed to Test Timing - Contractor License".to_string(),
        "Test Wind Meet - 6/6/2025 to 6/7/2025".to_string(),
        "Boys Long Jump Division 1".to_string(),
        SEP.to_string(),
        WIND_POINTS_HEADER.to_string(),
        SEP.to_string(),
        "Finals".to_string(),
    ];
    lines.extend(rows.iter().cloned());
    super::super::parse(&lines, source()).ok_or_else(|| "synthetic meet parses".to_string())
}

#[test]
fn jump_rows_keep_explicit_wind_and_never_invent_heats() -> TestResult {
    let meet = wind_meet(&[
        "  1 Legal Jumper             12 Test School               23-06   1.9  10   ".to_string(),
        "  2 Assisted Jumper          11 Test School               22-01   3.4   8    ".to_string(),
        "  3 Unknown Wind             10 Test School               21-00   NWI   6    ".to_string(),
    ])?;
    check!(eq; meet.events.len(), 1);
    let rows = &meet.events[0].rows;
    check!(eq; rows.len(), 3);
    check!(eq; rows[0].wind_mps, Some(1.9), "a legal wind is preserved");
    check!(eq; rows[1].wind_mps, Some(3.4), "an assisted wind is preserved, not dropped");
    check!(eq; rows[2].wind_mps, None, "NWI stays unknown instead of becoming a heat");
    for row in rows {
        check!(eq; row.heat, None, "field rows must not invent heats from wind or points");
    }
    check!(eq; rows[0].points, Some(10.0));
    check!(eq; rows[1].points, Some(8.0));
    check!(eq; rows[2].points, Some(6.0));
    check!(
        matches!(&rows[0].mark, Mark::FieldImperial { .. }),
        "the imperial mark survives beside its wind: {:?}",
        rows[0].mark
    );
    Ok(())
}

#[test]
fn triple_jump_rows_keep_explicit_wind_without_flight_confusion() -> TestResult {
    let lines = vec![
        "Licensed to Test Timing - Contractor License".to_string(),
        "Test Wind Meet - 6/6/2025 to 6/7/2025".to_string(),
        "Girls Triple Jump Division 1".to_string(),
        SEP.to_string(),
        WIND_POINTS_HEADER.to_string(),
        SEP.to_string(),
        "Finals".to_string(),
        "  1 Triple One               12 Test School               38-00   2.0  10   ".to_string(),
        "  2 Triple Two               11 Test School               36-06   NWI   8    ".to_string(),
    ];
    let parsed =
        super::super::parse(&lines, source()).ok_or_else(|| "synthetic meet parses".to_string())?;
    check!(eq; parsed.events.len(), 1);
    check!(
        matches!(&parsed.events[0].kind, EventKind::TripleJump),
        "triple jump maps to its kind: {:?}",
        parsed.events[0].kind
    );
    let rows = &parsed.events[0].rows;
    check!(eq; rows.len(), 2);
    check!(eq; rows[0].wind_mps, Some(2.0));
    check!(eq; rows[1].wind_mps, None);
    for row in rows {
        check!(eq; row.heat, None, "wind must not leak into flight");
    }
    Ok(())
}

#[test]
fn points_only_races_carry_no_wind_and_no_heat() -> TestResult {
    let lines = vec![
        "Licensed to Test Timing - Contractor License".to_string(),
        "Test Points Meet - 6/6/2025 to 6/7/2025".to_string(),
        "Boys 100 Meter Dash Division 1".to_string(),
        SEP.to_string(),
        POINTS_ONLY_HEADER.to_string(),
        SEP.to_string(),
        "Finals".to_string(),
        "  1 Sprinter One             12 Test School               10.89  10    ".to_string(),
        "  2 Sprinter Two             11 Test School               10.95   8    ".to_string(),
        "  3 Sprinter Three           10 Test School               11.02   6    ".to_string(),
    ];
    let parsed =
        super::super::parse(&lines, source()).ok_or_else(|| "synthetic meet parses".to_string())?;
    let rows = &parsed.events[0].rows;
    check!(eq; rows.len(), 3);
    for row in rows {
        check!(eq; row.wind_mps, None, "no Wind column means unknown wind");
        check!(eq; row.heat, None, "scoring points must not become heats");
    }
    check!(eq; rows[0].points, Some(10.0));
    check!(eq; rows[1].points, Some(8.0));
    check!(eq; rows[2].points, Some(6.0));
    Ok(())
}

fn finals_rows(
    header: &str,
    rows: &[String],
) -> Result<Vec<crate::result_file::ParsedRow>, String> {
    let mut lines = vec![
        "Licensed to Test Timing - Contractor License".to_string(),
        "Test Conflict Meet - 6/6/2025 to 6/7/2025".to_string(),
        "Boys 100 Meter Dash Division 1".to_string(),
        SEP.to_string(),
        header.to_string(),
        SEP.to_string(),
        "Finals".to_string(),
    ];
    lines.extend(rows.iter().cloned());
    let parsed =
        super::super::parse(&lines, source()).ok_or_else(|| "synthetic meet parses".to_string())?;
    Ok(parsed.events[0].rows.clone())
}

#[test]
fn contradictory_finals_marks_from_two_sources_share_one_context() -> TestResult {
    let header = WIND_POINTS_HEADER;
    let first = finals_rows(
        header,
        &[
            "  1 Ben Lemirand              12 West De Pere             10.57   1.0  10   "
                .to_string(),
            "  2 Ryan Heiman               11 Arrowhead                10.59   1.0   8   "
                .to_string(),
        ],
    )?;
    let second = finals_rows(
        header,
        &[
            "  1 Ben Lemirand              12 West De Pere             10.62   1.0  10   "
                .to_string(),
            "  2 Ryan Heiman               11 Arrowhead                10.59   1.0   8   "
                .to_string(),
        ],
    )?;
    check!(eq; first.len(), 2);
    check!(eq; second.len(), 2);
    for (left, right) in first.iter().zip(second.iter()) {
        check!(eq; left.heat, None);
        check!(eq; right.heat, None);
        check!(eq; left.heat, right.heat, "one final is one context in both sources");
    }
    check!(
        first[0].mark != second[0].mark,
        "the two sources genuinely contradict on the winning mark"
    );
    check!(eq; first[1].mark, second[1].mark, "the runner-up mark agrees");
    Ok(())
}
