//! The meet half of the reader: a meet index page is [`MeetRef`]s plus its pager, and a meet page
//! lists the result files an athlete's marks live in.

use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

use super::super::wire::{MeetRef, MeetResultFile};
use super::{
    MEET_INDEX_MARKER_REGEX, MEET_ROW_DAY_REGEX, MEET_ROW_ID_REGEX, MEET_ROW_LINK_REGEX,
    MEET_ROW_VENUE_REGEX,
};

fn meet_index_marker_regex() -> CrawlResult<&'static Regex> {
    MEET_INDEX_MARKER_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_INDEX_MARKER_REGEX",
            source: source.clone(),
        })
}

/// The `meetResultFiles = [ … ]` literal a meet's results page embeds: every result file the meet
/// has, each with its own id. The capture runs to the first `]` and is then parsed as JSON, so a
/// body whose literal is not the JSON the site publishes fails as a schema mismatch rather than
/// being scraped loosely.
static MEET_RESULT_FILES_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)meetResultFiles\s*=\s*(\[[^]]*\])"));

fn meet_result_files_regex() -> CrawlResult<&'static Regex> {
    MEET_RESULT_FILES_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_RESULT_FILES_REGEX",
            source: source.clone(),
        })
}

/// Read the result files a meet's results page lists, in the order the page lists them.
///
/// An empty list is a meet with no published results yet, which is a state rather than an error; a
/// page that publishes no `meetResultFiles` literal at all is a schema mismatch, because that page
/// is the only cheap place the list exists.
pub fn parse_meet_result_files(html: &str) -> CrawlResult<Vec<MeetResultFile>> {
    let regex = meet_result_files_regex()?;
    let Some(captured) = regex.captures(html) else {
        return Err(CrawlError::Schema {
            url: "meet results page".to_string(),
            detail: "no meetResultFiles literal in the body".to_string(),
        });
    };
    let literal = captured
        .get(1)
        .map(|match_| match_.as_str())
        .unwrap_or_default();
    serde_json::from_str(literal).map_err(|source| CrawlError::Decode {
        url: "meet results page".to_string(),
        source,
    })
}

fn meet_row_id_regex() -> CrawlResult<&'static Regex> {
    MEET_ROW_ID_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_ROW_ID_REGEX",
            source: source.clone(),
        })
}

fn meet_row_link_regex() -> CrawlResult<&'static Regex> {
    MEET_ROW_LINK_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_ROW_LINK_REGEX",
            source: source.clone(),
        })
}

fn meet_row_day_regex() -> CrawlResult<&'static Regex> {
    MEET_ROW_DAY_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_ROW_DAY_REGEX",
            source: source.clone(),
        })
}

fn meet_row_venue_regex() -> CrawlResult<&'static Regex> {
    MEET_ROW_VENUE_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_ROW_VENUE_REGEX",
            source: source.clone(),
        })
}

/// Read a state results index into one row per meet, in the order the page publishes them.
///
/// The month a row sits under is carried across the scan because the row itself publishes only a
/// day (`Sep 19`) while the section publishes only the bucket (`2026-09`); a row before the first
/// section, or one whose day is not a day of that month, keeps `date: None` instead of borrowing a
/// neighbour's month. A row with no meet id or no results link is dropped: this reader's contract is
/// a requestable meet, so a row it cannot address is not one.
pub fn parse_meet_index(html: &str) -> CrawlResult<Vec<MeetRef>> {
    let marker = meet_index_marker_regex()?;
    let id = meet_row_id_regex()?;
    let link = meet_row_link_regex()?;
    let day = meet_row_day_regex()?;
    let venue = meet_row_venue_regex()?;
    let mut meets = Vec::new();
    let mut month: Option<String> = None;
    for capture in marker.captures_iter(html) {
        if let Some(bucket) = capture.get(1) {
            month = Some(bucket.as_str().to_string());
            continue;
        }
        let Some(row) = capture.get(2).map(|row| row.as_str()) else {
            continue;
        };
        let Some(meet_id) = id
            .captures(row)
            .and_then(|captures| captures.get(1))
            .map(|capture| capture.as_str().to_string())
        else {
            continue;
        };
        let Some(link) = link.captures(row) else {
            continue;
        };
        let (Some(url), Some(name)) = (link.get(1), link.get(2)) else {
            continue;
        };
        let day = day
            .captures(row)
            .and_then(|captures| captures.get(1))
            .map(|capture| capture.as_str().to_string())
            .unwrap_or_default();
        meets.push(MeetRef {
            meet_id,
            name: html_unescape(name.as_str().trim()),
            date: month.as_deref().and_then(|bucket| iso_date(bucket, &day)),
            venue: venue
                .captures(row)
                .and_then(|captures| captures.get(1))
                .map(|capture| html_unescape(capture.as_str().trim()))
                .unwrap_or_default(),
            results_url: url.as_str().to_string(),
        });
    }
    Ok(meets)
}

/// Whether the index page publishes a next page (`rel="next"`, the pager's own marker).
pub fn has_next_page(html: &str) -> bool {
    html.contains(r#"rel="next""#)
}

/// `2026-09` plus `Sep 19` gives `2026-09-19`; `None` when the day is not a day of that month, or
/// when the month bucket is not a month.
fn iso_date(month_bucket: &str, day: &str) -> Option<String> {
    let (year, month) = month_bucket.split_once('-')?;
    let year: i32 = year.parse().ok()?;
    let month: u8 = month.parse().ok()?;
    let day: u8 = day
        .split_whitespace()
        .next_back()?
        .trim_end_matches(|ch: char| !ch.is_ascii_digit())
        .parse()
        .ok()?;
    if day == 0 || day > days_in_month(year, month)? {
        return None;
    }
    Some(format!("{year}-{month:02}-{day:02}"))
}

/// The month's length, refusing a month number no calendar has.
fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

pub(in crate::milesplit) fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}
