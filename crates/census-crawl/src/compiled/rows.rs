//! Row reading: the row a block's cells describe, the relay legs printed beneath a relay row, and
//! the mark a row publishes.

use regex::Regex;
use std::sync::LazyLock;

use census_domain::model::{EventKind, Mark};

use crate::hytek::{self, grade_from_token, looks_like_a_name, substring};
use crate::{CrawlError, CrawlResult};

use super::layout::Block;
use super::{ParsedEvent, ParsedRow, RelayLeg};

/// The qualifier letter a preliminary row carries after its mark (`12.30 Q`).
static QUALIFIER: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"\s+[Qq]$"));
static RELAY_LEG: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^\d]+?)\s+(\d{1,2}|Fr|So|Jr|Sr)\b"));

fn qualifier() -> CrawlResult<&'static Regex> {
    QUALIFIER.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "QUALIFIER",
        source: source.clone(),
    })
}

fn relay_leg() -> CrawlResult<&'static Regex> {
    RELAY_LEG.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "RELAY_LEG",
        source: source.clone(),
    })
}

pub(super) fn starts_like_a_row(slice: &str) -> bool {
    slice
        .trim_start()
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
}

pub(super) fn parse_row(
    line: &str,
    line_tokens: &[hytek::Token<'_>],
    block: &Block,
) -> Option<ParsedRow> {
    let first_column = block.columns.first()?.start;
    let place = line_tokens
        .iter()
        .rfind(|token| token.start >= block.start && token.end <= first_column.saturating_add(1))
        .and_then(|token| token.text.trim().parse::<u16>().ok());
    let place = place?;

    let kind = block.kind.clone();
    let raw = block.numeric(
        line_tokens,
        &[
            "Finals",
            "Prelims",
            "Preliminaries",
            "Time",
            "Mark",
            "Distance",
        ],
    );
    let mark = mark_for(&kind, raw?)?;

    let year = block
        .numeric(line_tokens, &["Yr", "Year"])
        .and_then(grade_from_token);
    let school = block.text(line, &["Team", "School"]).unwrap_or_default();
    let name = block.text(line, &["Athlete", "Name"]).unwrap_or_default();
    let points = block
        .numeric(line_tokens, &["Points", "Pts"])
        .and_then(|token| token.parse::<f64>().ok());

    if name.is_empty() && school.is_empty() {
        return None;
    }
    if !kind.is_relay() && (name.is_empty() || !looks_like_a_name(&name)) {
        return None;
    }
    Some(ParsedRow {
        place: Some(place),
        name,
        grade: year,
        school,
        mark,
        wind_mps: None,
        heat: None,
        points,
        legs: Vec::new(),
    })
}

/// Attach the relay legs a relay row prints beneath it. `None` means the leg pattern is
/// unavailable, which makes the file unreadable rather than legless.
pub(super) fn attach_legs(line: &str, block: &Block, event: &mut ParsedEvent) -> Option<bool> {
    if !block.kind.is_relay() {
        return Some(false);
    }
    let Some(row) = event.rows.last_mut() else {
        return Some(false);
    };
    let slice = substring(line, block.start, block.limit);
    let mut found = false;
    for captures in relay_leg().ok()?.captures_iter(&slice) {
        let name = captures
            .get(2)
            .map(|m| m.as_str().trim().trim_end_matches(','))
            .unwrap_or_default()
            .to_string();
        if !looks_like_a_name(&name) {
            continue;
        }
        let position = captures
            .get(1)
            .map(|m| m.as_str().parse::<u8>())
            .and_then(|r| r.ok())
            .unwrap_or(0);
        row.legs.push(RelayLeg {
            position,
            name,
            grade: captures
                .get(3)
                .and_then(|m| grade_from_token(m.as_str().trim())),
        });
        found = true;
    }
    Some(found)
}

/// Read the published mark: qualifier letters are not part of it, and jumps and throws publish feet
/// and inches rather than a time.
fn mark_for(kind: &EventKind, token: &str) -> Option<Mark> {
    let token = token.trim();
    if hytek::NO_MARK.contains(&token.to_ascii_uppercase().as_str()) {
        return Some(Mark::Raw(token.to_string()));
    }
    let cleaned = qualifier().ok()?.replace(token, "");
    let cleaned = cleaned.trim();
    if kind.is_field() {
        return hytek::parse_field_mark(cleaned)
            .or_else(|| hytek::parse_time(cleaned).map(Mark::TimeSeconds));
    }
    hytek::parse_time(cleaned)
        .map(Mark::TimeSeconds)
        .or_else(|| hytek::parse_field_mark(cleaned))
}
