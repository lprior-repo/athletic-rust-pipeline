//! An event header line's own vocabulary: the event number some exports print ahead of the name,
//! the round the header ends with, and the gender/label/division the header names.

use regex::Regex;
use std::sync::LazyLock;

use census_domain::model::Gender;

use crate::{CrawlError, CrawlResult};

/// Event numbers that some exports print ahead of the event name (`#22 Girls' 4x800 Relay`).
static EVENT_NUMBER: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\s*#\s?\d+\s+"));
static ROUND_TAIL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\s+(finals|prelims|preliminaries|semi-?finals|semis)\s*$"));
static EVENT_LABEL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^(Boys|Girls|Men|Women)['\u{2019}]?s?\s+(.+?)(?:\s+(Division\s+[0-9A-Za-z]+))?\s*$",
    )
});

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn event_number() -> CrawlResult<&'static Regex> {
    EVENT_NUMBER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "EVENT_NUMBER",
            source: source.clone(),
        })
}

fn round_tail() -> CrawlResult<&'static Regex> {
    ROUND_TAIL.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "ROUND_TAIL",
        source: source.clone(),
    })
}

fn event_label() -> CrawlResult<&'static Regex> {
    EVENT_LABEL
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "EVENT_LABEL",
            source: source.clone(),
        })
}

/// Gender, label, division and round of an event header block such as
/// `Girls' 4x800 Relay Division 1          Finals`.
pub(super) fn event_of(slice: &str) -> Option<(Gender, String, Option<String>, Option<String>)> {
    let trimmed = event_number().ok()?.replace(slice.trim(), "");
    let trimmed = trimmed.trim();
    let round_tail = round_tail().ok()?;
    let round = round_tail
        .captures(trimmed)
        .and_then(|captures| {
            let label = captures.get(1)?.as_str().to_ascii_lowercase();
            Some(match label.as_str() {
                "prelims" | "preliminaries" => "preliminaries",
                "semis" | "semi-finals" | "semifinals" => "semi-finals",
                _ => "finals",
            })
        })
        .map(str::to_string);
    let head = round_tail.replace(trimmed, "");
    let captures = event_label().ok()?.captures(head.trim())?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    if label.is_empty() {
        return None;
    }
    let division = captures
        .get(3)
        .map(|division| division.as_str().trim().to_string());
    Some((gender, label, division, round))
}
