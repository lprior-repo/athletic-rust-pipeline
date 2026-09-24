//! A `/raw` body's report, checked against itself and against the column map it documents.
//!
//! The measured column map (see `sources/milesplit/raw_rows/columns.rs`) is reproduced here as a
//! builder, so these laws are read off the documented layout rather than off one capture's bytes:
//! a row built to the published columns must be accepted, and a row one column too wide must be
//! *reported* rather than silently misread. That pair is the guarantee the run report's "lines
//! dropped by the column map" counter rests on.

use super::{
    parse_meet_index, parse_meet_result_files, parse_raw, OH_FILE_LIST, OH_RAW, OH_RAW_ROWS,
    OH_RAW_URL,
};

/// One fixed-width result line, built to the published column map: place `0..4`, a blank, athlete
/// `5..30`, a blank, grade `31..33`, a blank, team `34..74`, the separator at `74`, the mark
/// `75..84`, and the heat cell closing the line at `90..92`.
fn row_line(name: &str, grade: &str, team: &str, mark: &str) -> String {
    format!(
        "{:>4} {name:<25} {grade:>2} {team:<40} {mark:>9}{:6}{:>2}",
        "1",
        "",
        "",
        name = name,
        grade = grade,
        team = team,
        mark = mark,
    )
}

/// The capture with `line` published as one more row of its last section.
fn with_row(body: &str, line: &str) -> String {
    body.replacen("</pre>", &format!("\n{line}\n</pre>"), 1)
}

#[test]
fn the_capture_parses_the_rows_it_publishes_and_drops_none() {
    let page = parse_raw(OH_RAW, OH_RAW_URL).expect("the capture parses");
    assert_eq!(page.meet.rows_parsed, OH_RAW_ROWS);
    assert!(
        page.skipped.is_empty(),
        "a verbatim capture must not drop a line: {:?}",
        page.skipped
    );
}

#[test]
fn the_rows_parsed_agree_with_the_rows_the_events_hold() {
    let page = parse_raw(OH_RAW, OH_RAW_URL).expect("the capture parses");
    let held: usize = page.meet.events.iter().map(|event| event.rows.len()).sum();
    assert_eq!(
        page.meet.rows_parsed, held,
        "every row the report counts is a row an event holds"
    );
    assert_eq!(
        page.meet.rows_skipped,
        page.skipped.len(),
        "the count and the list of skips are two views of one thing"
    );
}

#[test]
fn a_row_built_to_the_published_columns_is_accepted() {
    let line = row_line("Jeydyn Fields", "8", "Jackson", "12:40.6");
    assert_eq!(
        line.chars().count(),
        92,
        "the builder emits the published width"
    );
    let body = with_row(OH_RAW, &line);
    let page = parse_raw(&body, OH_RAW_URL).expect("the capture with one more row parses");
    assert_eq!(
        page.meet.rows_parsed,
        OH_RAW_ROWS + 1,
        "a row on the published columns is a row"
    );
    assert!(
        page.skipped.is_empty(),
        "nothing was dropped: {:?}",
        page.skipped
    );
}

/// A row whose mark is one character too long, encoded the way the vendor's fixed-width writer
/// encodes one: right-aligned into the nine-column cell, so the mark's own first character lands in
/// the separator column beside it. This is the case `columns.rs` names as the reason the separator
/// guard exists.
fn row_with_overflowing_mark() -> String {
    const MARK: &str = "1:23:45.67";
    const CELL_END: usize = 84;
    let mut chars: Vec<char> = row_line("Jeydyn Fields", "8", "Jackson", "")
        .chars()
        .collect();
    for (offset, ch) in MARK.chars().enumerate() {
        chars[CELL_END - MARK.chars().count() + offset] = ch;
    }
    chars.into_iter().collect()
}

#[test]
fn a_row_whose_mark_overflows_its_columns_is_reported_rather_than_misread() {
    let line = row_with_overflowing_mark();
    let body = with_row(OH_RAW, &line);
    let page = parse_raw(&body, OH_RAW_URL).expect("the capture with one bad row parses");
    assert_eq!(
        page.meet.rows_parsed, OH_RAW_ROWS,
        "the shifted row is not read as a row"
    );
    assert_eq!(
        page.skipped.len(),
        1,
        "it is reported instead: {:?}",
        page.skipped
    );
    assert!(
        page.skipped[0].contains("did not fit the column map"),
        "the report names why: {:?}",
        page.skipped[0]
    );
    assert_eq!(page.meet.rows_skipped, 1);
}

#[test]
fn every_result_file_derives_a_raw_url_the_reader_accepts() {
    let files = parse_meet_result_files(super::OH_FILE_LIST_URL, OH_FILE_LIST)
        .expect("the file list parses");
    assert!(!files.is_empty(), "the capture lists result files");
    let page = parse_meet_index(super::OH_INDEX).expect("the index parses");
    let meet = page
        .iter()
        .find(|meet| meet.meet_id == "770621")
        .expect("the capture's meet is in the index");
    let host = meet
        .results_url
        .split('/')
        .nth(2)
        .expect("the results URL names a host");
    for file in &files {
        let raw = file.raw_url(&meet.results_url);
        let parsed = super::ResultSetRef::parse(&raw)
            .unwrap_or_else(|| panic!("{raw} is not a results URL the arm can request"));
        assert_eq!(parsed.meet_id, meet.meet_id, "the file names its own meet");
        assert_eq!(
            parsed.rsid,
            file.id.to_string(),
            "the file names its own id"
        );
        assert!(
            raw.starts_with(&format!("https://{host}/meets/{}-", meet.meet_id)),
            "the derived URL stays under the listing page's host and meet: {raw}"
        );
    }
}
