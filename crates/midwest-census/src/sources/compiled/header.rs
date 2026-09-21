//! Meet name and date from the print header: the page stamp, the readable date stamp, and the
//! print artifacts that are not part of the name.

use regex::Regex;
use std::sync::LazyLock;

use crate::sources::{CrawlError, CrawlResult};

static PAGE_STAMP: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4},\s*\d{1,2}:\d{2}\s*(?:AM|PM)\s*"));
static DATE_NAMED: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)[a-z]*,?\s+([A-Z][a-z]{2,8})\s+(\d{1,2}),\s*(\d{4})",
    )
});
static DATE_SLASH: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b"));
static VENUE_NOISE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(high school|hs)\b|,\s*[A-Z]{2}\s*$"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn page_stamp() -> CrawlResult<&'static Regex> {
    PAGE_STAMP.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "PAGE_STAMP",
        source: source.clone(),
    })
}

fn date_named() -> CrawlResult<&'static Regex> {
    DATE_NAMED.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_NAMED",
        source: source.clone(),
    })
}

fn date_slash() -> CrawlResult<&'static Regex> {
    DATE_SLASH.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_SLASH",
        source: source.clone(),
    })
}

fn venue_noise() -> CrawlResult<&'static Regex> {
    VENUE_NOISE
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "VENUE_NOISE",
            source: source.clone(),
        })
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Meet name and date from the print header.
///
/// The first line is a page stamp (`5/27/25, 8:35 PM`) that may also carry the meet name; the venue
/// line carries the readable date stamp (`Tue, May 27, 2025`). Chrome's print header occasionally
/// prepends `Manage ` to the name, which is dropped because no meet is called that.
pub(super) fn header(lines: &[String]) -> Option<(String, Option<String>)> {
    let mut name: Option<String> = None;
    let mut date: Option<String> = None;
    for line in lines.iter().take(8) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if date.is_none() {
            date = parse_date(trimmed);
        }
        if name.is_none() {
            let cleaned = page_stamp().ok()?.replace(trimmed, "");
            let cleaned = cleaned.trim().trim_start_matches("Manage ").trim();
            let candidate = date_named().ok()?.replace(cleaned, "");
            let candidate = date_slash().ok()?.replace(&candidate, "");
            let candidate = candidate.trim().trim_end_matches('|').trim();
            if candidate.len() >= 6
                && !venue_noise().ok()?.is_match(candidate)
                && !candidate.eq_ignore_ascii_case("results")
                && candidate.chars().any(char::is_alphabetic)
            {
                name = Some(candidate.to_string());
            }
        }
        if name.is_some() && date.is_some() {
            break;
        }
    }
    Some((name?, date))
}

fn parse_date(line: &str) -> Option<String> {
    if let Some(captures) = date_named().ok()?.captures(line) {
        let month_token = captures.get(1)?.as_str().to_ascii_lowercase();
        let month = MONTHS
            .iter()
            .position(|month| month_token.starts_with(month))?
            .checked_add(1)?;
        return Some(format!(
            "{:04}-{:02}-{:02}",
            captures.get(3)?.as_str().parse::<i32>().ok()?,
            month,
            captures.get(2)?.as_str().parse::<u32>().ok()?
        ));
    }
    let captures = date_slash().ok()?.captures(line)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        captures.get(3)?.as_str().parse::<i32>().ok()?,
        captures.get(1)?.as_str().parse::<u32>().ok()?,
        captures.get(2)?.as_str().parse::<u32>().ok()?
    ))
}
