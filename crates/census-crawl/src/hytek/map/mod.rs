//! Meet, event and mark mapping for Hy-Tek reports.
//!
//! The meet header, the event labels and the published marks are what a Hy-Tek report states in
//! prose instead of in fixed columns; the readers here turn each into the canonical shape the
//! model carries: an ISO date, an event kind, a round marker, or a `Mark` with its wind and heat.

use crate::result_file::ParsedEvent;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Gender};
use regex::Regex;
use std::sync::LazyLock;

static EVENT_HEADER: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"^(?:Event\s+\d+\s+)?(Boys|Girls)\s+(.+?)(?:\s+(Division\s+[0-9A-Za-z]+))?\s*$")
});

static MARK_TOKEN: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^[0-9][0-9:.\-]*[A-Za-z]?$"));

static DATED: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^(.*?)\s+-\s+(\d{1,2})/(\d{1,2})/(\d{4})(?:\s+to\s+(\d{1,2})/(\d{1,2})/(\d{4}))?\s*$",
    )
});

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn event_header_regex() -> CrawlResult<&'static Regex> {
    EVENT_HEADER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "EVENT_HEADER",
            source: source.clone(),
        })
}

pub(super) fn mark_token_regex() -> CrawlResult<&'static Regex> {
    MARK_TOKEN.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "MARK_TOKEN",
        source: source.clone(),
    })
}

fn dated_regex() -> CrawlResult<&'static Regex> {
    DATED.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATED",
        source: source.clone(),
    })
}

/// Plain-text marks that are results rather than numbers.
pub(crate) const NO_MARK: [&str; 8] = ["DNF", "DNS", "SCR", "NH", "FOUL", "NM", "DQ", "X"];

/// The meet name and dates are published on one header line, either as a single day
/// (`Name - 6/6/2025`) or as a range (`Name - 6/6/2025 to 6/7/2025`).
pub(super) fn header_meet(lines: &[String]) -> Option<(String, String, Option<String>)> {
    let dated = dated_regex().ok()?;
    for line in lines.iter().take(40) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Licensed to") {
            continue;
        }
        let Some(captures) = dated.captures(trimmed) else {
            continue;
        };
        let name = captures.get(1)?.as_str().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let iso = |month: &str, day: &str, year: &str| -> Option<String> {
            Some(format!(
                "{:04}-{:02}-{:02}",
                year.parse::<i32>().ok()?,
                month.parse::<u32>().ok()?,
                day.parse::<u32>().ok()?
            ))
        };
        let start = iso(
            captures.get(2)?.as_str(),
            captures.get(3)?.as_str(),
            captures.get(4)?.as_str(),
        )?;
        let end = match (captures.get(5), captures.get(6), captures.get(7)) {
            (Some(month), Some(day), Some(year)) => {
                iso(month.as_str(), day.as_str(), year.as_str())
            }
            _ => None,
        };
        return Some((name, start, end));
    }
    None
}

pub(super) fn event_header(trimmed: &str) -> Option<ParsedEvent> {
    let captures = event_header_regex().ok()?.captures(trimmed)?;
    let gender = match captures.get(1)?.as_str() {
        "Boys" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    let kind = hytek_event_kind(&label);
    // The pattern also matches prose (`Boys of summer 2025`), so only labels the ontology knows, or
    // labels shaped like a track event, become events.
    if matches!(kind, EventKind::Unmapped { .. }) && !looks_like_event(&label) {
        return None;
    }
    Some(ParsedEvent {
        label,
        kind,
        gender,
        division: captures.get(3).map(|m| m.as_str().trim().to_string()),
        round: None,
        rows: Vec::new(),
    })
}

fn looks_like_event(label: &str) -> bool {
    let lowered = label.to_ascii_lowercase();
    [
        "meter", "metre", "jump", "vault", "put", "discus", "relay", "hurdle", "steeple", "medley",
        "athlon", "throw", "javelin", "hammer", "run", "dash", "walk",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

mod mark;

pub(super) use mark::parse_marks;
pub use mark::{hytek_event_kind, parse_field_mark, parse_time, round_marker};
