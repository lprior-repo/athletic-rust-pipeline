use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

mod bounded;
mod rows;
pub(in crate::milesplit) use bounded::parse_bounded;

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

static MEET_RESULT_FILES_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)meetResultFiles\s*=\s*(\[[^]]*\])"));

static DD_RESULTS_PAGE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<select\s+id="ddResultsPage">\s*(.*?)\s*</select>"#));

static LEGACY_OPTION_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"<option\s+value="https?://[^/]+/meets/\d+-[^"]+/results/(\d+)(?:\?[^"]*)?"[^>]*>([^<]+)</option>"#,
    )
});

const INLINE_RESULTS_ANCHOR: &str = r#"id="meetResultsBody""#;

fn meet_result_files_regex() -> CrawlResult<&'static Regex> {
    MEET_RESULT_FILES_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "MEET_RESULT_FILES_REGEX",
            source: source.clone(),
        })
}

fn legacy_fields(option_html: &str) -> Option<(i64, &str)> {
    let re = LEGACY_OPTION_REGEX.as_ref().ok()?;
    let cap = re.captures(option_html)?;
    let id: i64 = cap.get(1)?.as_str().parse().ok()?;
    let name = cap.get(2)?.as_str().trim();
    (!name.is_empty() && id > 0).then_some((id, name))
}

pub fn parse_meet_result_files(url: &str, html: &str) -> CrawlResult<Vec<MeetResultFile>> {
    use crate::milesplit::results::pages::bounded::{Allocation, MeetPageLimits, MAX_FILES};
    let mut allocation = Allocation::new(MeetPageLimits::standard());
    parse_bounded(url, html, MAX_FILES, &mut allocation)
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

pub fn parse_meet_index(html: &str) -> CrawlResult<Vec<MeetRef>> {
    rows::parse(html)
}

pub fn has_next_page(html: &str) -> bool {
    html.contains(r#"rel="next""#)
}

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

fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => Some(29),
        2 => Some(28),
        _ => None,
    }
}
