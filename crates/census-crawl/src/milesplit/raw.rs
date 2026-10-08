use crate::result_file::ParsedMeet;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{SchoolYear, Sport};
use serde::Deserialize;

use super::raw_rows::{read_block, RawSection};

mod document;

enum DatePrecision {
    Day,
    Published,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawPage {
    pub meet: ParsedMeet,
    pub sport: Option<Sport>,
    pub region: Option<String>,
    pub school_year: SchoolYear,
    pub skipped: Vec<String>,
    pub grade_issues: Vec<super::RawGradeIssue>,
}

pub fn parse_raw(html: &str, url: &str) -> CrawlResult<RawPage> {
    parse_document(html, url, document::Ownership::Unbound, DatePrecision::Day)
}

pub(in crate::milesplit) fn parse_bound(
    html: &str,
    url: &str,
    check: &dyn Fn(&str, &str) -> CrawlResult<()>,
) -> CrawlResult<RawPage> {
    parse_document(
        html,
        url,
        document::Ownership::Bound(check),
        DatePrecision::Published,
    )
}

fn parse_document(
    html: &str,
    url: &str,
    ownership: document::Ownership<'_>,
    precision: DatePrecision,
) -> CrawlResult<RawPage> {
    let event = document::read(html, url, ownership)?;
    let facts = page_facts(event, url, precision)?;
    let (block_start, block_text) =
        pre_block(html).ok_or_else(|| schema(url, "no <pre> result block"))?;
    let block = read_block(block_text, facts.sport, block_start)?;
    if !block.qualified || block.sections.is_empty() {
        return Err(schema(
            url,
            &format!(
                "unqualified raw result document; rejections: {:?}",
                block.skipped
            ),
        ));
    }
    let events: Vec<_> = block
        .sections
        .iter()
        .map(RawSection::event)
        .collect::<Vec<_>>();
    Ok(RawPage {
        meet: ParsedMeet {
            name: facts.name,
            date: facts.date,
            end_date: facts.end_date,
            timer: None,
            events,
            rows_parsed: block.rows_parsed,
            rows_skipped: block.skipped.len(),
        },
        sport: facts.sport,
        region: facts.region,
        school_year: facts.school_year,
        skipped: block.skipped,
        grade_issues: block.grade_issues,
    })
}

struct PageFacts {
    name: String,
    date: String,
    end_date: Option<String>,
    sport: Option<Sport>,
    region: Option<String>,
    school_year: SchoolYear,
}

fn page_facts(event: SportsEvent, url: &str, precision: DatePrecision) -> CrawlResult<PageFacts> {
    let name = event.name.trim();
    if name.is_empty() {
        return Err(schema(url, "schema.org block carries no meet name"));
    }
    let sport = event.sport.as_deref().and_then(sport_of);
    let published = event.start_date.trim();
    let date = iso_day(published).ok_or_else(|| {
        schema(
            url,
            &format!("meet start date {published:?} is not an ISO day"),
        )
    })?;
    let school_year = school_year_of(date)
        .ok_or_else(|| schema(url, &format!("meet start date {date:?} has no month")))?;
    Ok(PageFacts {
        name: name.to_string(),
        date: match precision {
            DatePrecision::Day => date.to_string(),
            DatePrecision::Published => event.start_date,
        },
        end_date: event
            .end_date
            .as_deref()
            .and_then(iso_day)
            .map(str::to_string),
        sport,
        region: event
            .location
            .and_then(|location| location.address)
            .and_then(|address| address.region)
            .map(|region| region.trim().to_string()),
        school_year,
    })
}

fn iso_day(date: &str) -> Option<&str> {
    const DAY_LEN: usize = 10;
    const DASHES: [usize; 2] = [4, 7];
    let day = date.get(..DAY_LEN)?;
    let well_formed = day
        .chars()
        .enumerate()
        .all(|(index, ch)| match DASHES.contains(&index) {
            true => ch == '-',
            false => ch.is_ascii_digit(),
        });
    if !well_formed {
        return None;
    }
    let month: u8 = day.get(5..7)?.parse().ok()?;
    (1..=12).contains(&month).then_some(day)
}

fn school_year_of(day: &str) -> Option<SchoolYear> {
    let year: i16 = day.get(..4)?.parse().ok()?;
    let month: u8 = day.get(5..7)?.parse().ok()?;
    SchoolYear::containing(year, month)
}

fn schema(url: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct SportsEvent {
    name: String,
    #[serde(rename = "startDate")]
    start_date: String,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    sport: Option<String>,
    location: Option<Location>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Location {
    address: Option<Address>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Address {
    #[serde(rename = "addressRegion")]
    region: Option<String>,
}

fn pre_block(html: &str) -> Option<(usize, &str)> {
    let (_, after_open) = html.split_once("<pre")?;
    let (_, body) = after_open.split_once('>')?;
    let start = html.len().checked_sub(body.len())?;
    let (block, _) = body.split_once("</pre>")?;
    Some((start, block))
}

fn sport_of(value: &str) -> Option<Sport> {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("cross") {
        Some(Sport::CrossCountry)
    } else if lowered.contains("indoor") {
        Some(Sport::IndoorTrack)
    } else if lowered.contains("track") || lowered.contains("outdoor") {
        Some(Sport::OutdoorTrack)
    } else {
        None
    }
}
