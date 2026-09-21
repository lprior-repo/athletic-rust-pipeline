//! The entry point: one RaceDay export body in, one parsed meet out.

use crate::sources::result_file::{ParsedEvent, ParsedMeet};
use crate::sources::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, SourceRef};

use super::regexes::{body_regex, cell_regex, head_regex, row_regex, table_regex, tags_regex};
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

    let table_pattern = table_regex()?;
    let head_pattern = head_regex()?;
    let row_pattern = row_regex()?;
    let cell_pattern = cell_regex()?;
    let body_pattern = body_regex()?;

    let mut events = Vec::new();
    for table in table_pattern.find_iter(body).map(|m| m.as_str()) {
        let labels = table_labels(table, tags, head_pattern, row_pattern, cell_pattern);
        let rows = table_rows(
            table,
            &labels,
            tags,
            row_pattern,
            cell_pattern,
            body_pattern,
        );
        if rows.is_empty() {
            continue;
        }
        events.push(ParsedEvent {
            label: "Cross Country".to_string(),
            kind: EventKind::CrossCountry,
            gender,
            division: division.clone(),
            round: Some("finals".to_string()),
            rows,
        });
    }
    if events.is_empty() {
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
        events,
        rows_parsed: 0,
        rows_skipped: 0,
    })
}
