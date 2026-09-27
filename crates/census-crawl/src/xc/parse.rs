use crate::result_file::{ParsedEvent, ParsedMeet};
use census_domain::model::SourceRef;

use super::header::header;
use super::scan::XcScan;

pub fn parse(lines: &[String], source: SourceRef, archive_year: i16) -> Option<ParsedMeet> {
    let (name, date) = header(lines)?;
    let date = date.unwrap_or_else(|| archive_year.to_string());
    let mut scan = XcScan::new();
    for line in lines {
        scan.read_line(line)?;
    }

    let events: Vec<ParsedEvent> = scan
        .events
        .into_iter()
        .filter(|event| !event.rows.is_empty())
        .collect();
    if events.is_empty() {
        return None;
    }
    let _ = source;
    Some(ParsedMeet {
        name,
        date,
        end_date: None,
        timer: None,
        events,
        rows_parsed: scan.rows_parsed,
        rows_skipped: scan.rows_skipped,
    })
}
