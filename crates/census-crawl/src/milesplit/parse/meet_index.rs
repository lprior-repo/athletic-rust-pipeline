use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

use super::super::wire::{MeetRef, MeetResultFile};
use super::html_unescape;
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

fn legacy_option(option_html: &str) -> Option<(i64, String)> {
    let re = LEGACY_OPTION_REGEX.as_ref().ok()?;
    let cap = re.captures(option_html)?;
    let id: i64 = cap.get(1)?.as_str().parse().ok()?;
    let name = cap.get(2)?.as_str().trim().to_string();
    (!name.is_empty() && id > 0).then_some((id, name))
}

fn parse_legacy_results_page(html: &str) -> Option<Vec<MeetResultFile>> {
    let re = DD_RESULTS_PAGE_REGEX.as_ref().ok()?;
    let select_html = re.captures(html)?.get(1)?.as_str();
    let mut files = Vec::new();
    for option_match in LEGACY_OPTION_REGEX
        .as_ref()
        .ok()?
        .captures_iter(select_html)
    {
        if let Some((id, name)) = legacy_option(option_match.get(0)?.as_str()) {
            files.push(MeetResultFile {
                id,
                name,
                is_meet_pro: 0,
                inline: false,
            });
        }
    }
    Some(files)
}

pub fn parse_meet_result_files(url: &str, html: &str) -> CrawlResult<Vec<MeetResultFile>> {
    let regex = meet_result_files_regex()?;
    let Some(captured) = regex.captures(html) else {
        if let Some(files) = parse_legacy_results_page(html) {
            return Ok(files);
        }
        if html.contains(INLINE_RESULTS_ANCHOR) {
            return Ok(vec![MeetResultFile {
                id: 0,
                name: "Results".to_string(),
                is_meet_pro: 0,
                inline: true,
            }]);
        }
        return Err(CrawlError::Schema {
            url: url.to_string(),
            detail: "no meetResultFiles literal, ddResultsPage select or meetResultsBody block"
                .to_string(),
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
