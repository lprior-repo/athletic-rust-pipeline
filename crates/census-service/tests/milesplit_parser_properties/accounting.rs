use super::{
    parse_meet_index, parse_meet_result_files, parse_raw, NC_RAW, NC_RAW_ROWS, NC_RAW_URL,
    OH_FILE_LIST,
};
use census_crawl::result_file::ParsedRow;
use census_domain::model::{CentiSeconds, Grade, Mark};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn with_row(body: &str, line: &str) -> String {
    body.replacen("</pre>", &format!("\n{line}\n</pre>"), 1)
}

fn capture_row_line(capture: &str) -> TestResult<String> {
    capture
        .lines()
        .find(|line| line.starts_with("   1 "))
        .map(|line| format!("{line}\n"))
        .ok_or_else(|| "the capture publishes no first-place row".into())
}

#[test]
fn the_capture_parses_the_rows_it_publishes_and_drops_none() -> TestResult {
    let page = parse_raw(NC_RAW, NC_RAW_URL)?;
    check!(eq; page.meet.rows_parsed, NC_RAW_ROWS);
    check!(
        page.skipped.is_empty(),
        "a verbatim capture must not drop a line: {:?}",
        page.skipped
    );
    Ok(())
}

#[test]
fn the_rows_parsed_agree_with_the_rows_the_events_hold() -> TestResult {
    let page = parse_raw(NC_RAW, NC_RAW_URL)?;
    let held: usize = page.meet.events.iter().map(|event| event.rows.len()).sum();
    check!(eq; page.meet.rows_parsed, held,
    "every row the report counts is a row an event holds");
    check!(eq; page.meet.rows_skipped,
    page.skipped.len(),
    "the count and the list of skips are two views of one thing");
    Ok(())
}

#[test]
fn a_row_on_the_captures_published_columns_is_a_row() -> TestResult {
    let line = capture_row_line(NC_RAW)?;
    let body = with_row(NC_RAW, &line);
    let page = parse_raw(&body, NC_RAW_URL)?;
    check!(eq; page.meet.rows_parsed,
    NC_RAW_ROWS + 1,
    "a row on the capture's own columns is a row");
    check!(
        page.skipped.is_empty(),
        "nothing was dropped: {:?}",
        page.skipped
    );
    Ok(())
}

#[test]
fn every_result_file_derives_a_raw_url_the_reader_accepts() -> TestResult {
    let files = parse_meet_result_files(super::OH_FILE_LIST_URL, OH_FILE_LIST)?;
    check!(!files.is_empty(), "the capture lists result files");
    let page = parse_meet_index(super::OH_INDEX)?;
    let meet = page
        .iter()
        .find(|meet| meet.meet_id == "770621")
        .ok_or("capture meet missing from index")?;
    let host = meet
        .results_url
        .split('/')
        .nth(2)
        .ok_or("results URL carries no host")?;
    for file in &files {
        let raw = file.raw_url(&meet.results_url);
        let parsed = super::ResultSetRef::parse(&raw)
            .ok_or_else(|| format!("{raw} is not a requestable results URL"))?;
        check!(eq; parsed.meet_id, meet.meet_id, "the file names its own meet");
        check!(eq; parsed.rsid,
        file.id.to_string(),
        "the file names its own id");
        check!(
            raw.starts_with(&format!("https://{host}/meets/{}-", meet.meet_id)),
            "the derived URL stays under the listing page's host and meet: {raw}"
        );
    }
    Ok(())
}

#[test]
fn the_captures_rows_keep_their_published_fields() -> TestResult {
    let page = parse_raw(NC_RAW, NC_RAW_URL)?;
    let rows: Vec<&ParsedRow> = page
        .meet
        .events
        .iter()
        .flat_map(|event| event.rows.iter())
        .collect();

    let winner = rows
        .iter()
        .find(|row| row.name == "FERGUSON, Michael")
        .ok_or("missing first-place row")?;
    check!(eq; winner.place, Some(1));
    check!(eq; winner.school, "North Buncombe");
    check!(eq; winner.grade, Some(Grade::new(12).ok_or("invalid grade")?));
    check!(eq; winner.mark, Mark::TimeSeconds(CentiSeconds::new(52473)));

    let eighth = rows
        .iter()
        .find(|row| row.name == "SURFACE, Luke")
        .ok_or("missing single-character grade row")?;
    check!(eq; eighth.place, Some(20));
    check!(eq; eighth.school, "North Raleigh Christ");
    check!(eq; eighth.grade, None,
    "an eighth grader is outside the domain's high-school grades");
    check!(eq; eighth.mark, Mark::TimeSeconds(CentiSeconds::new(54250)));

    let unplaced = rows
        .iter()
        .find(|row| row.name == "WHARTON, Elijah")
        .ok_or("missing unplaced row")?;
    check!(eq; unplaced.place, None);
    check!(eq; unplaced.school, "Davidson Academy");
    check!(eq; unplaced.mark, Mark::Raw("DNF".to_string()));
    Ok(())
}
