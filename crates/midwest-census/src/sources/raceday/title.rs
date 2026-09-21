//! Race titles: the export's `<h3>`, the race name carried in it, and the division it names.

use crate::sources::{CrawlError, CrawlResult};
use regex::Regex;

use super::regexes::title_regex;
use super::table::text_of;

/// The text of the export's `<h3>`, which is the race title.
pub(super) fn race_title(body: &str, tags: &Regex) -> CrawlResult<String> {
    title_regex()?
        .captures(body)
        .and_then(|captures| captures.get(1))
        .map(|m| text_of(tags, m.as_str()))
        .filter(|title| !title.is_empty())
        .ok_or_else(|| CrawlError::Invariant {
            detail: "no title found in RaceDay export".to_string(),
        })
}

/// `WIAA D2 XC Sectionals - Boys Race Team Finish List-XC` → `WIAA D2 XC Sectionals - Boys Race`.
pub(super) fn race_name(title: &str) -> CrawlResult<String> {
    let trimmed = title
        .trim_end_matches("-XC")
        .trim_end_matches("Team Finish List")
        .trim_end_matches("Team Summary")
        .trim_end_matches("Individual Results")
        .trim()
        .to_string();
    if trimmed.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no race name extracted from title".to_string(),
        });
    }
    Ok(trimmed)
}

/// `WIAA D2 XC Sectionals` → `Division 2`.
pub(super) fn division_of(title: &str) -> Option<String> {
    let lowered = title.to_ascii_lowercase();
    if let Some(index) = lowered.find("division ") {
        let start = index.saturating_add("division ".len());
        let rest = title.get(start..).unwrap_or_default();
        let token: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric())
            .collect();
        if !token.is_empty() {
            return Some(format!("Division {token}"));
        }
    }
    for token in title.split_whitespace() {
        let candidate = token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        let digits: String = candidate
            .chars()
            .skip_while(|ch| !ch.is_ascii_digit())
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if !digits.is_empty() && candidate.to_ascii_lowercase().starts_with('d') {
            return Some(format!("Division {digits}"));
        }
    }
    None
}
