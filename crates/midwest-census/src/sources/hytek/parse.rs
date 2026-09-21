//! Section and row reading for the Hy-Tek fixed-column report.
//!
//! One results section states its own layout in its header line, and every row beneath it is read
//! against that layout: place left of the first labelled column, the school or name text columns
//! sliced to the next numeric anchor, the mark and its trailing wind, heat and points columns.

use crate::sources::result_file::{ParsedRow, RelayLeg};
use census_domain::model::{EventKind, Grade};
use regex::Regex;
use std::sync::LazyLock;

use super::columns::{
    columns_from_header, looks_like_a_name, substring, tokens, Column, Token, TEXT_LABELS,
};
use super::map::parse_marks;

static RELAY_LEG: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^0-9]+?)\s+(\d{1,2})\b"));
static PLACE_PREFIX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d+\)"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn relay_leg_regex() -> anyhow::Result<&'static Regex> {
    RELAY_LEG
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

pub(super) fn place_prefix_regex() -> anyhow::Result<&'static Regex> {
    PLACE_PREFIX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

/// Columns that carry a result mark, in the order a mark is taken when a row publishes several.
/// `Seed` is deliberately absent: an entry mark is not a performance.
const MARK_LABELS: [&str; 10] = [
    "Finals",
    "Time",
    "Result",
    "Results",
    "Mark",
    "Prelims",
    "Preliminaries",
    "Semi-Finals",
    "Best",
    "Distance",
];

/// The column layout of the section currently being read.
#[derive(Debug, Clone, Default)]
pub(super) struct Section {
    columns: Vec<Column>,
    /// Round implied by the section's mark column (`Prelims` → `preliminaries`).
    pub(super) round: Option<&'static str>,
}

impl Section {
    /// Read a section header. Returns `None` for anything that is not a results header: the line
    /// must begin with a text column and publish at least one mark column, which keeps team-score
    /// and split tables out of the row stream.
    pub(super) fn from_header(header: &str) -> Option<Section> {
        let trimmed = header.trim_start();
        if !TEXT_LABELS.iter().any(|label| trimmed.starts_with(label)) {
            return None;
        }
        let columns = columns_from_header(header);
        if !columns
            .iter()
            .any(|column| MARK_LABELS.contains(&column.label.as_str()))
        {
            return None;
        }
        let round = columns
            .iter()
            .find_map(|column| match column.label.as_str() {
                "Prelims" | "Preliminaries" => Some("preliminaries"),
                "Semi-Finals" | "Semis" => Some("semi-finals"),
                "Finals" => Some("finals"),
                _ => None,
            });
        Some(Section { columns, round })
    }

    fn column(&self, label: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.label == label)
    }

    /// The token printed under `column`: numeric columns are right-aligned to the label's edge, so
    /// the match is on the token that ends there.
    fn numeric_token_for<'a>(&self, tokens: &[Token<'a>], column: &Column) -> Option<Token<'a>> {
        // The right-aligned token meets the label's edge, within the single space that separates
        // the columns; the bound saturates instead of wrapping past the end of the line.
        let edge = column.end.saturating_add(1);
        tokens
            .iter()
            .filter(|token| token.end.saturating_add(1) >= column.end && token.end <= edge)
            .min_by_key(|token| column.end.abs_diff(token.end))
            .copied()
    }

    fn numeric_token<'a>(&self, tokens: &[Token<'a>], label: &str) -> Option<Token<'a>> {
        self.column(label)
            .and_then(|column| self.numeric_token_for(tokens, column))
    }

    /// Where the value of the next numeric column starts — the end boundary of the text column that
    /// precedes it.
    fn next_numeric_start(&self, tokens: &[Token<'_>], offset: usize) -> Option<usize> {
        self.columns
            .iter()
            .filter(|column| column.numeric)
            .filter_map(|column| self.numeric_token_for(tokens, column))
            .filter(|token| token.start > offset)
            .map(|token| token.start)
            .min()
    }

    fn heat<'a>(&self, tokens: &[Token<'a>]) -> Option<String> {
        ["H#", "Flight", "Lane"]
            .iter()
            .find_map(|label| self.numeric_token(tokens, label))
            .map(|token| token.text.to_string())
    }
}

pub(super) fn starts_like_a_row(trimmed: &str) -> bool {
    trimmed
        .split_whitespace()
        .next()
        .is_some_and(|token| token.chars().all(|ch| ch.is_ascii_digit()))
}

pub(super) fn parse_row(line: &str, kind: &EventKind, section: &Section) -> Option<ParsedRow> {
    let tokens = tokens(line);
    let (place, name, school, grade) = row_identity(line, &tokens, section)?;

    // The mark is a published result column, never the `Seed` entry mark.
    let mut marks = None;
    for label in MARK_LABELS {
        let Some(token) = section.numeric_token(&tokens, label) else {
            continue;
        };
        let tail = line.get(token.end..).unwrap_or_default();
        if let Some(parsed) = parse_marks(kind, token.text, tail) {
            marks = Some(parsed);
            break;
        }
    }
    let (mark, wind_mps, heat_from_tail, points_from_tail) = marks?;
    let heat = section.heat(&tokens).or(heat_from_tail);
    let points = section
        .numeric_token(&tokens, "Points")
        .and_then(|token| token.text.parse::<f64>().ok())
        .or(points_from_tail);

    Some(ParsedRow {
        place,
        name,
        grade,
        school,
        mark,
        wind_mps,
        heat,
        points,
        legs: Vec::new(),
    })
}

/// Place, name, school and grade of one row — the identity columns every layout publishes.
///
/// `None` is the line that is not an athlete row: a row whose section names no school column, a
/// blank or bracketed school, or a name that does not read as one.
fn row_identity(
    line: &str,
    tokens: &[Token<'_>],
    section: &Section,
) -> Option<(Option<u16>, String, String, Option<Grade>)> {
    let first_column_start = section.columns.first()?.start;

    // Place sits left of the first labelled column and is optional (unranked rows print blank).
    let place = tokens
        .iter()
        .rfind(|token| token.end <= first_column_start)
        .and_then(|token| token.text.parse::<u16>().ok());

    let name_start = section.column("Name").map(|column| column.start);
    // Relay sections print the school where individual sections print the athlete; `Team` and
    // `Relay` are the same idea under other names.
    let school_start = ["School", "Team", "Relay", "Athlete"]
        .iter()
        .find_map(|label| section.column(label).map(|column| column.start))
        .or(name_start)?;

    let school = match section.next_numeric_start(tokens, school_start) {
        Some(end) => substring(line, school_start, end),
        None => substring(line, school_start, line.len()),
    };
    let name = match name_start {
        Some(start) => {
            let end = section
                .next_numeric_start(tokens, start)
                .unwrap_or(school_start);
            substring(line, start, end)
        }
        None => String::new(),
    };
    if school.is_empty() || school.contains(')') {
        return None;
    }
    if !name.is_empty() && !looks_like_a_name(&name) {
        return None;
    }
    let grade = section
        .numeric_token(tokens, "Year")
        .and_then(|token| token.text.parse::<u8>().ok())
        .and_then(Grade::new);

    Some((place, name, school, grade))
}

pub(super) fn parse_legs(trimmed: &str, filled: usize) -> Vec<RelayLeg> {
    let Ok(relay_leg) = relay_leg_regex() else {
        return Vec::new();
    };
    let mut legs = Vec::new();
    for captures in relay_leg.captures_iter(trimmed) {
        // The fallback counts the legs already on the row; it saturates rather than wrapping, so a
        // leg is never renumbered to 0 the way a truncating cast would.
        let fallback =
            u8::try_from(filled.saturating_add(legs.len()).saturating_add(1)).unwrap_or(u8::MAX);
        let position: u8 = captures
            .get(1)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(fallback);
        let name = captures
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let grade = captures
            .get(3)
            .and_then(|m| m.as_str().parse::<u8>().ok())
            .and_then(census_domain::model::Grade::new);
        if name.is_empty() {
            continue;
        }
        legs.push(RelayLeg {
            position,
            name,
            grade,
        });
    }
    legs
}
