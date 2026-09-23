//! The `/raw` page reader: the meet identity from the page's schema.org block, the result rows from
//! its fixed-width `<pre>` block.
//!
//! Measured on `research/sources/milesplit-national/samples/raw-oh-770621-rs1321880.txt`, the body of
//! `https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw`
//! (2026-09-22T03:55:59Z, HTTP 200, 47,564 B): **80 result rows in 2 sections, in one request** —
//! no pagination, no second call, and every row carries place, athlete, `Yr` (grade), team and mark.
//!
//! The formatted sibling is not an alternative. `samples/meet-oh-770621-results.html` (the
//! `/meets/770621-beaver-eastern-invite-2026/results` body, canonical
//! `…/results/1321880/formatted`) renders **0 result rows**: its table is built in the browser from
//! `v1/meets/<meetId>/performances` on `api.prod.milesplit.com` (`js-loadresultsnew.js`,
//! `js-api.js`). `/api/` is `Disallow`ed for every agent in every captured `robots.txt`
//! (`robots-oh-milesplit.txt:7`), so this adapter never requests `/api/`, never requests the API
//! host, and never requests `/rankings`, `/virtual-meets` or `/contact`.
//!
//! What the file does not carry, and this reader therefore does not invent: no `AthleteID`, no
//! `TeamID`, no timing method, and no `MeetID` inside the text — the meet's identity comes from the
//! schema.org block, and the ids that name the result set come from the URL the caller supplied
//! ([`super::wire::ResultSetRef`]).

use crate::result_file::ParsedMeet;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{SchoolYear, Sport};
use serde::Deserialize;

use super::raw_rows::{read_block, RawSection};

/// A `/raw` page: the shared result-file shape plus the meet facts the fixed-width text omits.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPage {
    /// The meet, its date and its events — the identity every canonical entity of the result set is
    /// stamped with.
    pub meet: ParsedMeet,
    /// The sport the page's schema.org block names, when it names one.
    pub sport: Option<Sport>,
    /// The `addressRegion` the page publishes (`"OH"`), which the caller cross-checks against the
    /// site it requested: the host is the identity channel, this is a published field.
    pub region: Option<String>,
    /// The school year the meet's start date sits in, which every row's grade is dated by.
    pub school_year: SchoolYear,
    /// Lines that carried a mark but did not fit the column map, as reported.
    pub skipped: Vec<String>,
}

/// Read a `/raw` body.
pub fn parse_raw(html: &str, url: &str) -> CrawlResult<RawPage> {
    let facts = page_facts(html, url)?;
    let block_text = pre_block(html).ok_or_else(|| schema(url, "no <pre> result block"))?;
    let block = read_block(block_text, facts.sport)?;
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
            // The file names the meet's host, not the timing vendor; no timer is published here.
            timer: None,
            events,
            rows_parsed: block.rows_parsed,
            rows_skipped: block.skipped.len(),
        },
        sport: facts.sport,
        region: facts.region,
        school_year: facts.school_year,
        skipped: block.skipped,
    })
}

/// The meet facts the page's schema.org block publishes.
struct PageFacts {
    name: String,
    date: String,
    end_date: Option<String>,
    sport: Option<Sport>,
    region: Option<String>,
    school_year: SchoolYear,
}

/// The page's schema.org block, decoded: name, dates, sport and region, with the start date checked
/// into a school year so a row's grade always has a season to sit in.
fn page_facts(html: &str, url: &str) -> CrawlResult<PageFacts> {
    let block = ld_block(html).ok_or_else(|| schema(url, "no schema.org SportsEvent block"))?;
    let event: SportsEvent = serde_json::from_str(block)
        .map_err(|source| schema(url, &format!("schema.org block did not decode: {source}")))?;
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
        date: date.to_string(),
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

/// The day part of a published date, when it is a `YYYY-MM-DD` day within a real month.
///
/// A schema.org `startDate` may carry a full timestamp; the day is what a meet identity and a school
/// year need, and a date of another shape — or a month outside `1..=12` — is a shape error rather
/// than something to trim into place.
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

/// The school year an ISO day sits in: the day's own month is what places it in a season.
fn school_year_of(day: &str) -> Option<SchoolYear> {
    let year: i16 = day.get(..4)?.parse().ok()?;
    let month: u8 = day.get(5..7)?.parse().ok()?;
    SchoolYear::containing(year, month)
}

/// A shape error tagged with the URL the body was fetched from.
fn schema(url: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct SportsEvent {
    name: String,
    #[serde(rename = "startDate")]
    start_date: String,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    sport: Option<String>,
    location: Option<Location>,
}

#[derive(Debug, Deserialize)]
struct Location {
    address: Option<Address>,
}

#[derive(Debug, Deserialize)]
struct Address {
    #[serde(rename = "addressRegion")]
    region: Option<String>,
}

/// The `<pre>` payload of a `/raw` page, without the tag that opens it or the one that closes it.
///
/// The page carries one `<pre>` block (measured: the first `<pre` in the capture opens the first
/// section header), so the reader takes the first `<pre` up to its `>` and the `</pre>` that closes
/// it, and reports a page that has none rather than reading part of the page as result text.
fn pre_block(html: &str) -> Option<&str> {
    let (_, after_open) = html.split_once("<pre")?;
    let (_, body) = after_open.split_once('>')?;
    let (block, _) = body.split_once("</pre>")?;
    Some(block)
}

/// The JSON of the page's schema.org block.
fn ld_block(html: &str) -> Option<&str> {
    const OPEN: [&str; 2] = [
        "<script type=\"application/ld+json\">",
        "<script type='application/ld+json'>",
    ];
    OPEN.iter().find_map(|open| {
        let (_, after_open) = html.split_once(open)?;
        let (block, _) = after_open.split_once("</script>")?;
        Some(block)
    })
}

/// The sport a schema.org `sport` string names, when it names one.
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
