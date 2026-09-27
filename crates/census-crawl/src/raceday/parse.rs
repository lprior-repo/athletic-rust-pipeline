
use crate::result_file::{ParsedEvent, ParsedMeet};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender, SourceRef};
use regex::Regex;

use super::regexes::{tags_regex, Patterns};
use super::table::{table_labels, table_rows};
use super::title::{division_of, race_name, race_title};

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
        return Err(CrawlError::Schema {
            url: source.url.unwrap_or(source.id),
            detail: "no events parsed from RaceDay export".to_string(),
        });
    }
    let rows_skipped = tables.rejected.iter().map(|r| r.len()).sum();
    Ok(ParsedMeet {
        name,
        date: format!("{year:04}"),
        end_date: None,
        timer: Some("RaceDay Scoring".to_string()),
        events: tables.events,
        rows_parsed: tables.rows_parsed,
        rows_skipped,
    })
}

#[derive(Default)]
struct Tables {
    events: Vec<ParsedEvent>,
    rows_parsed: usize,
    rejected: Vec<Vec<super::table::RowRejection>>,
}

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
        let (rows, rejections) = table_rows(
            table,
            &labels,
            tags,
            patterns.row,
            patterns.cell,
            patterns.body,
        );
        if let Some(first) = rejections.first() {
            tracing::debug!(
                rows = rejections.len(),
                reason = first.reason.as_str(),
                "declined the data rows of a grid"
            );
            tables.rejected.push(rejections);
        }
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
