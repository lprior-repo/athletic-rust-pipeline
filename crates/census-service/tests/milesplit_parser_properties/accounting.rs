use super::{
    parse_meet_index, parse_meet_result_files, parse_raw, NC_RAW, NC_RAW_ROWS, NC_RAW_URL,
    OH_FILE_LIST,
};

fn with_row(body: &str, line: &str) -> String {
    body.replacen("</pre>", &format!("\n{line}\n</pre>"), 1)
}

fn capture_row_line(capture: &str) -> String {
    capture
        .lines()
        .find(|line| line.starts_with("   1 "))
        .map(|line| format!("{line}\n"))
        .unwrap_or_else(|| panic!("the capture publishes a first-place row"))
}

#[test]
fn the_capture_parses_the_rows_it_publishes_and_drops_none() {
    let page = parse_raw(NC_RAW, NC_RAW_URL).expect("the capture parses");
    assert_eq!(page.meet.rows_parsed, NC_RAW_ROWS);
    assert!(
        page.skipped.is_empty(),
        "a verbatim capture must not drop a line: {:?}",
        page.skipped
    );
}

#[test]
fn the_rows_parsed_agree_with_the_rows_the_events_hold() {
    let page = parse_raw(NC_RAW, NC_RAW_URL).expect("the capture parses");
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
fn a_row_on_the_captures_published_columns_is_a_row() {
    let line = capture_row_line(NC_RAW);
    let body = with_row(NC_RAW, &line);
    let page = parse_raw(&body, NC_RAW_URL).expect("the capture with one more row parses");
    assert_eq!(
        page.meet.rows_parsed,
        NC_RAW_ROWS + 1,
        "a row on the capture's own columns is a row"
    );
    assert!(
        page.skipped.is_empty(),
        "nothing was dropped: {:?}",
        page.skipped
    );
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
