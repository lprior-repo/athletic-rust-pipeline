//! The adapter entry point: read every line, keep the events that carry rows.
use crate::result_file::{ParsedEvent, ParsedMeet};
use census_domain::model::SourceRef;

use super::header::header;
use super::scan::XcScan;

/// Parse a cross-country result file.
///
/// `archive_year` supplies the school year when the file publishes no date at all: the sectional
/// family from TrackSide prints a date in no header and no footer, and a year is enough to place the
/// race in the right school year (the same floor the RaceDay parser uses).
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
    // The source reference is part of the parser chain's uniform signature; a parsed meet
    // carries no source field of its own.
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
