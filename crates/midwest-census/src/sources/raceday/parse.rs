//! The entry point: one RaceDay export body in, one parsed meet out.

use crate::sources::result_file::{ParsedEvent, ParsedMeet};
use crate::sources::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, SourceRef};
use regex::Regex;

use super::regexes::{tags_regex, Patterns};
use super::table::{table_labels, table_rows};
use super::title::{division_of, race_name, race_title};

/// Parse a RaceDay export. `year` is the season the file was archived under, used because the format
/// publishes no date of its own.
pub fn parse(body: &str, source: SourceRef, year: i16) -> CrawlResult<ParsedMeet> {
    let tags = tags_regex()?;
    let title = race_title(body, tags)?;
    let name = race_name(&title)?;
    let gender = if title.to_ascii_lowercase().contains("girls") {
        Gender::Girls
    } else {
        Gender::Boys
    };
    let division = division_of(&title);

    let patterns = Patterns::compile()?;
    let tables = read_tables(body, tags, &patterns, gender, division);
    if tables.events.is_empty() {
        // The archive page a body was read from is the only identity a body carries; the source id
        // names the provider when the artifact URL is unknown.
        return Err(CrawlError::Schema {
            url: source.url.unwrap_or(source.id),
            detail: "no events parsed from RaceDay export".to_string(),
        });
    }
    Ok(ParsedMeet {
        name,
        date: format!("{year:04}"),
        end_date: None,
        timer: Some("RaceDay Scoring".to_string()),
        events: tables.events,
        rows_parsed: tables.rows_parsed,
        rows_skipped: tables.rows_skipped,
    })
}

/// What one body's tables yielded: the events they became and the two counters the report
/// publishes.
///
/// `rows_parsed` counts every row the events carry, and `rows_skipped` counts the data rows of a
/// name-bearing table the reader declined because the row had no athlete cell or no time to take as
/// the mark.
#[derive(Default)]
struct Tables {
    events: Vec<ParsedEvent>,
    rows_parsed: usize,
    rows_skipped: usize,
}

/// Read every result table of one body into a cross-country event.
fn read_tables(
    body: &str,
    tags: &Regex,
    patterns: &Patterns,
    gender: Gender,
    division: Option<String>,
) -> Tables {
    let mut tables = Tables::default();
    for table in patterns.table.find_iter(body).map(|m| m.as_str()) {
        let labels = table_labels(table, tags, patterns.head, patterns.row, patterns.cell);
        let (rows, skipped) = table_rows(
            table,
            &labels,
            tags,
            patterns.row,
            patterns.cell,
            patterns.body,
        );
        tables.rows_skipped = tables.rows_skipped.saturating_add(skipped);
        if rows.is_empty() {
            continue;
        }
        tables.rows_parsed = tables.rows_parsed.saturating_add(rows.len());
        tables.events.push(ParsedEvent {
            label: "Cross Country".to_string(),
            kind: EventKind::CrossCountry,
            gender,
            division: division.clone(),
            round: Some("finals".to_string()),
            rows,
        });
    }
    tables
}
