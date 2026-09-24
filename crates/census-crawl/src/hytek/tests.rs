use super::*;
use census_domain::model::{EventKind, Gender, Grade, Mark, SourceRef};
use census_domain::model::CentiSeconds;

/// Verbatim slices of
/// `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm`
/// (WIAA Division 1 boys state championships, 2025-06-06, PrimeTime Timing): the report header
/// plus the 100 m dash section, and the header plus the 4x100 relay and shot put sections.
const DASH: &str = include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm");
const SECTIONS: &str =
    include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn parse_html(body: &str) -> ParsedMeet {
    let lines = lines_from_html(body);
    super::parse(&lines, source()).expect("fixture has a meet header")
}

#[test]
fn the_trackside_template_parses_place_grade_school_and_field_marks() {
    // TrackSide Timing publishes the `Name / Year / School / Finals / H# / Points` header and
    // right-aligned field marks (`115-10`, `J5-11.00`, `NH`); PrimeTime publishes `Grade` and
    // `Time`. Both templates come out of the same archive.
    let body = include_str!("../../tests/fixtures/wiaa_results/trackside-regional.htm");
    let parsed = parse(&lines_from_html(body), source()).expect("fixture has a meet header");
    assert_eq!(parsed.name, "WIAA Division 1 Badger Regional");
    assert_eq!(parsed.date, "2025-05-27");
    let discus = parsed
        .events
        .iter()
        .find(|event| event.kind == EventKind::Discus && event.gender == Gender::Girls)
        .unwrap_or_else(|| panic!("the girls discus event survives: {:?}", parsed.events));
    assert_eq!(discus.rows.len(), 17, "every placed thrower is a row");
    let winner = &discus.rows[0];
    assert_eq!(winner.name, "Ashlin Nottestad");
    assert_eq!(winner.grade.map(Grade::get), Some(12));
    assert_eq!(winner.school, "Badger");
    assert_eq!(winner.place, Some(1));
    assert!(
        matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark.starts_with("115")),
        "the discus mark keeps its published notation: {:?}",
        winner.mark
    );
}

#[test]
fn a_seed_column_does_not_bleed_into_the_school_label_or_the_mark() {
    // Sections that publish a `Seed` column print two field marks side by side. Reading the
    // school to a fixed offset swallowed the seed (`Flambeau  36-11.00`), which then failed
    // school resolution and silently dropped every placed thrower in the section.
    let body = include_str!("../../tests/fixtures/wiaa_results/seed-column-regional.htm");
    let parsed = parse(&lines_from_html(body), source()).expect("fixture has a meet header");
    let shot = parsed
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut && event.gender == Gender::Girls)
        .expect("the girls shot put survives");
    assert_eq!(shot.rows.len(), 22, "every placed thrower is a row");
    let winner = &shot.rows[0];
    assert_eq!(winner.name, "Roehl, Reese");
    assert_eq!(
        winner.school, "Flambeau",
        "the seed mark stays out of the label"
    );
    assert_eq!(winner.place, Some(1));
    assert_eq!(winner.grade.map(Grade::get), Some(11));
    assert!(
        matches!(&winner.mark, Mark::FieldImperial { feet_mark, .. } if feet_mark == "35-09.00"),
        "the mark is the published result, not the seed: {:?}",
        winner.mark
    );
    assert_eq!(winner.points, Some(10.0));
    // Labels stay clean for every row, which is what school resolution depends on.
    assert!(
        shot.rows
            .iter()
            .all(|row| !row.school.chars().any(|ch| ch.is_ascii_digit())),
        "no school label carries a mark: {:?}",
        shot.rows
            .iter()
            .map(|row| row.school.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn uppercase_pre_blocks_parse_like_plain_text() {
    // WIAA's older releases wrap the whole report in one uppercase `<PRE>` block instead of
    // one `<P>` per line; both shapes must yield the same report lines.
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
    // `pdftotext` separates pages with a form feed; a page footer must not join the next page's
    // first line, or the header and the first result row fuse into one unparsable line.
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
    assert_eq!(parse_time("10.56"), Some(CentiSeconds(1056)));
    assert_eq!(parse_time("1:54.32"), Some(CentiSeconds(11432)));
    assert_eq!(parse_time("15:32.1"), Some(CentiSeconds(93210)));
    assert_eq!(parse_time("DNF"), None);
    match parse_field_mark("61-03.50") {
        Some(Mark::FieldImperial { metres, .. }) => {
            assert!(
                (metres.0 - 1868).abs() < 1,
                "61'3.5\" is 18.68 m, got {metres}"
            );
        }
        other => panic!("expected an imperial field mark, got {other:?}"),
    }
}

#[test]
fn header_lines_yield_the_meet_name_date_and_timer() {
    let meet = parse_html(DASH);
    assert_eq!(meet.name, "WIAA Track & Field State Championships");
    assert_eq!(meet.date, "2025-06-06");
    assert_eq!(meet.timer.as_deref(), Some("PrimeTime Timing"));
}

#[test]
fn individual_rows_carry_place_grade_school_mark_and_wind() {
    let meet = parse_html(DASH);
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Track100m)
        .expect("the fixture publishes the 100 m dash");
    assert_eq!(event.gender, Gender::Boys);
    assert_eq!(event.division.as_deref(), Some("Division 1"));
    assert_eq!(event.round.as_deref(), Some("preliminaries"));
    let winner = event
        .rows
        .iter()
        .find(|row| row.place == Some(1))
        .expect("a first place row exists");
    assert_eq!(winner.name, "Ben Lemirand");
    assert_eq!(winner.grade.map(Grade::get), Some(12));
    assert_eq!(winner.school, "West De Pere");
    assert_eq!(winner.mark, Mark::TimeSeconds(CentiSeconds(1056)));
    assert_eq!(winner.wind_mps, Some(0.4));
    // Every prelim row carries a grade in this section; the parser must not invent one.
    assert!(event.rows.iter().all(|row| row.grade.is_some()));
    assert!(event.rows.len() >= 20, "got {} rows", event.rows.len());
}

#[test]
fn field_rows_keep_the_imperial_mark_and_the_flight() {
    let meet = parse_html(SECTIONS);
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut)
        .expect("the fixture publishes the shot put");
    assert_eq!(event.round.as_deref(), Some("finals"));
    let winner = event
        .rows
        .iter()
        .find(|row| row.place == Some(1))
        .expect("a first place row exists");
    assert_eq!(winner.name, "Hunter Sprangers");
    assert_eq!(winner.school, "Kimberly");
    match &winner.mark {
        Mark::FieldImperial { feet_mark, .. } => assert_eq!(feet_mark, "61-03.50"),
        other => panic!("expected an imperial mark, got {other:?}"),
    }
    assert_eq!(winner.points, Some(10.0));
}

#[test]
fn relay_rows_name_the_school_and_list_their_legs_with_grades() {
    let meet = parse_html(SECTIONS);
    let event = meet
        .events
        .iter()
        .find(|event| event.kind == EventKind::Relay4x100)
        .expect("the fixture publishes the 4x100 relay");
    let winner = event
        .rows
        .iter()
        .find(|row| row.school == "Homestead")
        .expect("Homestead ran the relay");
    assert!(
        winner.name.is_empty(),
        "relay rows name a school, not an athlete"
    );
    // Hy-Tek lists the four legs and then any alternates, each numbered as published.
    assert!(
        winner.legs.len() >= 4,
        "at least the four legs are listed, got {:?}",
        winner.legs
    );
    assert_eq!(winner.legs[0].name, "Jamir Erving");
    assert_eq!(winner.legs[0].position, 1);
    assert_eq!(winner.legs[0].grade.map(Grade::get), Some(11));
    assert_eq!(winner.legs[1].name, "Sean O'Byrne");
    assert_eq!(winner.legs[1].grade.map(Grade::get), Some(12));
    assert_eq!(winner.legs[2].name, "Jackson Montgomery");
    assert_eq!(winner.legs[3].name, "Lucas Mersky");
}

#[test]
fn team_score_lines_are_not_mistaken_for_results() {
    // `  21) Green Bay Preble            11       22) Holmen                     10` is a team
    // score table, not an individual result.
    let meet = parse_html(DASH);
    for event in &meet.events {
        for row in &event.rows {
            assert!(
                !row.school.contains(')'),
                "team score lines must not become results: {row:?}"
            );
            assert!(row.place.is_some());
        }
    }
}

#[test]
fn a_file_without_a_meet_header_is_skipped_rather_than_guessed() {
    let lines = lines_from_html("<html><body><p>Some other document</p></body></html>");
    assert_eq!(super::parse(&lines, source()), None);
}

/// The same report the HTML fixture came from, as the plain-text file Hy-Tek also publishes.
const DASH_TEXT: &str =
    include_str!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.txt");

#[test]
fn plain_text_reports_parse_the_same_way_as_html_ones() {
    let from_html = parse_html(DASH);
    let lines = lines_from_text(DASH_TEXT);
    let from_text = super::parse(&lines, source()).expect("text reports carry the same header");
    assert_eq!(from_text.name, from_html.name);
    assert_eq!(from_text.date, from_html.date);
    assert_eq!(from_text.rows_parsed, from_html.rows_parsed);
    assert_eq!(from_text.events.len(), from_html.events.len());
    assert_eq!(from_text.events[0].rows[0], from_html.events[0].rows[0]);
}
