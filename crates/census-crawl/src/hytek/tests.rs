use super::*;
use census_domain::model::CentiSeconds;
use census_domain::model::{EventKind, Gender, Grade, Mark, SourceRef};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod event_scores;

const DASH: &str = include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm");
const SECTIONS: &str =
    include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn parse_html(body: &str) -> TestResult<ParsedMeet> {
    let lines = lines_from_html(body);
    super::parse(&lines, source()).ok_or_else(|| "fixture has a meet header".into())
}

#[test]
fn the_trackside_template_parses_place_grade_school_and_field_marks() -> TestResult {
    let body = include_str!("../../tests/fixtures/wiaa_results/trackside-regional.htm");
    let parsed = parse(&lines_from_html(body), source()).ok_or("fixture has a meet header")?;
    check!(eq; parsed.name, "WIAA Division 1 Badger Regional");
    check!(eq; parsed.date, "2025-05-27");
    let discus = parsed
        .events
        .iter()
        .find(|event| event.kind == EventKind::Discus && event.gender == Gender::Girls)
        .ok_or("the girls discus event survives")?;
    check!(eq; discus.rows.len(), 17, "every placed thrower is a row");
    let winner = &discus.rows[0];
    check!(eq; winner.name, "Ashlin Nottestad");
    check!(eq; winner.grade.map(Grade::get), Some(12));
    check!(eq; winner.school, "Badger");
    check!(eq; winner.place, Some(1));
    check!(
        matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark.starts_with("115")),
        "the discus mark keeps its published notation: {:?}",
        winner.mark
    );
    Ok(())
}

#[test]
fn a_seed_column_does_not_bleed_into_the_school_label_or_the_mark() -> TestResult {
    let body = include_str!("../../tests/fixtures/wiaa_results/seed-column-regional.htm");
    let parsed = parse(&lines_from_html(body), source()).ok_or("fixture has a meet header")?;
    let shot = parsed
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut && event.gender == Gender::Girls)
        .ok_or("the girls shot put survives")?;
    check!(eq; shot.rows.len(), 22, "every placed thrower is a row");
    let winner = &shot.rows[0];
    check!(eq; winner.name, "Roehl, Reese");
    check!(eq;
        winner.school, "Flambeau",
        "the seed mark stays out of the label"
    );
    check!(eq; winner.place, Some(1));
    check!(eq; winner.grade.map(Grade::get), Some(11));
    check!(
        matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark == "35-09.00"),
        "the mark is the published result, not the seed: {:?}",
        winner.mark
    );
    check!(eq; winner.points, Some(10.0));
    check!(
        shot.rows
            .iter()
            .all(|row| !row.school.chars().any(|ch| ch.is_ascii_digit())),
        "no school label carries a mark: {:?}",
        shot.rows
            .iter()
            .map(|row| row.school.as_str())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn uppercase_pre_blocks_parse_like_plain_text() {
    let html = "<HTML>\r\n<BODY>\r\n<P>\r\n<PRE>\r\nLicensed to TrackSide\r\n\
               Event 3  Girls Discus Throw\r\n  1 Smith, Jane  11 Badger  120-03\r\n";
    let lines = lines_from_html(html);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Licensed to TrackSide")),
        "the meet header survives the `<PRE>` wrapper: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Smith, Jane")),
        "athlete rows survive the `<PRE>` wrapper: {lines:?}"
    );
}

#[test]
fn pdf_page_breaks_do_not_glue_pages_together() {
    let text = "Licensed to TrackSide\r\nEvent 3  Girls Discus Throw\r\n1 A, B  11  Badger\u{c}11/1/25, 12:38 PM\r\nLicensed to TrackSide\r\n";
    let lines = lines_from_pdf_text(text);
    assert!(
        lines.iter().any(|line| line == "1 A, B  11  Badger"),
        "the page footer is cut onto its own line: {lines:?}"
    );
    assert!(
        lines.iter().all(|line| !line.contains('\u{c}')),
        "no form feed survives into a report line: {lines:?}"
    );
}

#[test]
fn event_labels_map_onto_the_ontology() {
    assert_eq!(
        hytek_event_kind("100 Meter Dash"),
        EventKind::Track100m,
        "Hy-Tek spells the event out"
    );
    assert_eq!(hytek_event_kind("3200 Meter Run"), EventKind::Track3200m);
    assert_eq!(hytek_event_kind("4x200 Meter Relay"), EventKind::Relay4x200);
    assert_eq!(
        hytek_event_kind("4x800 Relay"),
        EventKind::Relay4x800,
        "the unit is optional in relay labels"
    );
    assert_eq!(
        hytek_event_kind("Sprint Medley Relay"),
        EventKind::SprintMedley
    );
    assert_eq!(hytek_event_kind("Shot Put"), EventKind::ShotPut);
    assert_eq!(hytek_event_kind("Discus Throw"), EventKind::Discus);
    assert_eq!(
        hytek_event_kind("110 Meter Hurdles"),
        EventKind::Track110mHurdles
    );
    assert_eq!(hytek_event_kind("Pole Vault"), EventKind::PoleVault);
    assert!(matches!(
        hytek_event_kind("300 Meter Hurdles Relay Race Unknown"),
        EventKind::Unmapped { .. }
    ));
}

#[test]
fn marks_parse_from_published_notation() {
    assert_eq!(parse_time("10.56"), Some(CentiSeconds::new(1056)));
    assert_eq!(parse_time("1:54.32"), Some(CentiSeconds::new(11432)));
    assert_eq!(parse_time("15:32.1"), Some(CentiSeconds::new(93210)));
    assert_eq!(parse_time("DNF"), None);
    match parse_field_mark("61-03.50") {
        Some(Mark::FieldImperial { metres, .. }) => {
            assert!(
                (metres.value() - 1868).abs() < 1,
                "61'3.5\" is 18.68 m, got {metres}"
            );
        }
        other => panic!("expected an imperial field mark, got {other:?}"),
    }
}

#[test]
fn header_lines_yield_the_meet_name_date_and_timer() -> TestResult {
    let meet = parse_html(DASH)?;
    check!(eq; meet.name, "WIAA Track & Field State Championships");
    check!(eq; meet.date, "2025-06-06");
    check!(eq; meet.timer.as_deref(), Some("PrimeTime Timing"));
    Ok(())
}

#[test]
fn individual_rows_carry_place_grade_school_mark_and_wind() -> TestResult {
    let meet = parse_html(DASH)?;
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Track100m)
        .ok_or("the fixture publishes the 100 m dash")?;
    check!(eq; event.gender, Gender::Boys);
    check!(eq; event.division.as_deref(), Some("Division 1"));
    check!(eq; event.round.as_deref(), Some("preliminaries"));
    let winner = event
        .rows
        .iter()
        .find(|row| row.place == Some(1))
        .ok_or("a first place row exists")?;
    check!(eq; winner.name, "Ben Lemirand");
    check!(eq; winner.grade.map(Grade::get), Some(12));
    check!(eq; winner.school, "West De Pere");
    check!(eq; winner.mark, Mark::TimeSeconds(CentiSeconds::new(1056)));
    check!(eq; winner.wind_mps, Some(0.4));
    check!(event.rows.iter().all(|row| row.grade.is_some()));
    check!(event.rows.len() >= 20, "got {} rows", event.rows.len());
    Ok(())
}

#[test]
fn field_rows_keep_the_imperial_mark_and_the_flight() -> TestResult {
    let meet = parse_html(SECTIONS)?;
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut)
        .ok_or("the fixture publishes the shot put")?;
    check!(eq; event.round.as_deref(), Some("finals"));
    let winner = event
        .rows
        .iter()
        .find(|row| row.place == Some(1))
        .ok_or("a first place row exists")?;
    check!(eq; winner.name, "Hunter Sprangers");
    check!(eq; winner.school, "Kimberly");
    match &winner.mark {
        Mark::FieldImperial { feet_mark, .. } => check!(eq; feet_mark, "61-03.50"),
        other => return Err(format!("expected an imperial mark, got {other:?}").into()),
    }
    check!(eq; winner.points, Some(10.0));
    Ok(())
}

#[test]
fn relay_rows_name_the_school_and_list_their_legs_with_grades() -> TestResult {
    let meet = parse_html(SECTIONS)?;
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .ok_or("the fixture publishes the 4x100 relay")?;
    let winner = event
        .rows
        .iter()
        .find(|row| row.school == "Homestead")
        .ok_or("Homestead ran the relay")?;
    check!(
        winner.name.is_empty(),
        "relay rows name a school, not an athlete"
    );
    check!(
        winner.legs.len() >= 4,
        "at least the four legs are listed, got {:?}",
        winner.legs
    );
    check!(eq; winner.legs[0].name, "Jamir Erving");
    check!(eq; winner.legs[0].position, 1);
    check!(eq; winner.legs[0].grade.map(Grade::get), Some(11));
    check!(eq; winner.legs[1].name, "Sean O'Byrne");
    check!(eq; winner.legs[1].grade.map(Grade::get), Some(12));
    check!(eq; winner.legs[2].name, "Jackson Montgomery");
    check!(eq; winner.legs[3].name, "Lucas Mersky");
    Ok(())
}

#[test]
fn team_score_lines_are_not_mistaken_for_results() -> TestResult {
    let meet = parse_html(DASH)?;
    for event in &meet.events {
        for row in &event.rows {
            check!(
                !row.school.contains(')'),
                "team score lines must not become results: {row:?}"
            );
            check!(row.place.is_some());
        }
    }
    Ok(())
}

#[test]
fn a_file_without_a_meet_header_is_skipped_rather_than_guessed() {
    let lines = lines_from_html("<html><body><p>Some other document</p></body></html>");
    assert_eq!(super::parse(&lines, source()), None);
}

const DASH_TEXT: &str =
    include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.txt");

#[test]
fn plain_text_reports_parse_the_same_way_as_html_ones() -> TestResult {
    let from_html = parse_html(DASH)?;
    let lines = lines_from_text(DASH_TEXT);
    let from_text = super::parse(&lines, source()).ok_or("text reports carry the same header")?;
    check!(eq; from_text.name, from_html.name);
    check!(eq; from_text.date, from_html.date);
    check!(eq; from_text.rows_parsed, from_html.rows_parsed);
    check!(eq; from_text.events.len(), from_html.events.len());
    check!(eq; from_text.events[0].rows[0], from_html.events[0].rows[0]);
    Ok(())
}
