use crate::result_file::{ParsedRow, RelayLeg};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventKind, Grade, TimingMethod};
use regex::Regex;
use std::sync::LazyLock;

use super::columns::{
    columns_from_header, looks_like_a_name, substring, tokens, Column, Token, TEXT_LABELS,
};
use super::identity::individual_identity;
use super::map::{parse_marks, time_is_hand};

static RELAY_LEG: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^0-9]+?)\s+(\d{1,2})\b"));
static PLACE_PREFIX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d+\)"));

fn relay_leg_regex() -> CrawlResult<&'static Regex> {
    RELAY_LEG.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "RELAY_LEG",
        source: source.clone(),
    })
}

pub(super) fn place_prefix_regex() -> CrawlResult<&'static Regex> {
    PLACE_PREFIX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PLACE_PREFIX",
            source: source.clone(),
        })
}

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

#[derive(Debug, Clone, Default)]
pub(super) struct Section {
    columns: Vec<Column>,
    pub(super) round: Option<&'static str>,
}

impl Section {
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

    fn numeric_token_for<'a>(&self, tokens: &[Token<'a>], column: &Column) -> Option<Token<'a>> {
        let edge = column.end.saturating_add(1);
        let previous_end = self
            .columns
            .iter()
            .filter(|other| other.start < column.start)
            .map(|other| other.end)
            .max()
            .unwrap_or_default();
        let trailing = !self.columns.iter().any(|other| other.start > column.start);
        tokens
            .iter()
            .filter(|token| {
                token.end <= edge
                    && (token.end.saturating_add(1) >= column.end
                        || (trailing && token.end > previous_end))
            })
            .min_by_key(|token| column.end.abs_diff(token.end))
            .copied()
    }

    fn numeric_token<'a>(&self, tokens: &[Token<'a>], label: &str) -> Option<Token<'a>> {
        self.column(label)
            .and_then(|column| self.numeric_token_for(tokens, column))
    }

    fn next_numeric_start(&self, tokens: &[Token<'_>], offset: usize) -> Option<usize> {
        self.columns
            .iter()
            .filter(|column| column.numeric)
            .filter_map(|column| self.numeric_token_for(tokens, column))
            .filter(|token| token.start > offset)
            .map(|token| token.start)
            .min()
    }

    fn heat(&self, tokens: &[Token<'_>]) -> Option<String> {
        ["H#", "Flight", "Lane"].iter().find_map(|label| {
            let column = self.column(label)?;
            let edge = column.end.saturating_add(1);
            let previous_end = self
                .columns
                .iter()
                .filter(|other| other.start < column.start)
                .map(|other| other.end)
                .max()
                .unwrap_or_default();
            let trailing = !self.columns.iter().any(|other| other.start > column.start);
            tokens
                .iter()
                .filter(|token| {
                    token.start >= column.start
                        && token.end <= edge
                        && (token.end.saturating_add(1) >= column.end
                            || (trailing && token.end > previous_end))
                        && heat_token(token.text)
                })
                .min_by_key(|token| column.end.abs_diff(token.end))
                .map(|token| token.text.to_string())
        })
    }
    fn wind(&self, tokens: &[Token<'_>]) -> Option<f64> {
        self.numeric_token(tokens, "Wind")
            .and_then(|token| token.text.parse::<f64>().ok())
            .filter(|value| value.is_finite() && value.abs() <= 12.0)
    }

    fn points(&self, tokens: &[Token<'_>]) -> Option<f64> {
        ["Points", "Pts"]
            .iter()
            .find_map(|label| self.numeric_token(tokens, label))
            .and_then(|token| token.text.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
    }
}

fn heat_token(text: &str) -> bool {
    !text.is_empty()
        && (text.chars().all(|ch| ch.is_ascii_digit())
            || (text.len() == 1 && text.chars().all(|ch| ch.is_ascii_alphabetic())))
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
    let (mark, mark_text) = row_mark(&tokens, section, kind)?;
    let heat = section.heat(&tokens);
    let points = section.points(&tokens);
    let wind = section.wind(&tokens);
    let timing = if time_is_hand(mark_text) {
        Some(TimingMethod::Hand)
    } else {
        None
    };

    Some(ParsedRow {
        place,
        name,
        grade,
        school,
        mark,
        timing,
        wind_mps: wind,
        heat,
        points,
        legs: Vec::new(),
    })
}

fn row_mark<'a>(
    tokens: &[Token<'a>],
    section: &Section,
    kind: &EventKind,
) -> Option<(Mark, &'a str)> {
    for label in MARK_LABELS {
        let Some(token) = section.numeric_token(tokens, label) else {
            continue;
        };
        if let Some(parsed) = parse_marks(kind, token.text) {
            return Some((parsed, token.text));
        }
    }
    None
}

fn row_place(tokens: &[Token<'_>], first_column_start: usize) -> Option<u16> {
    tokens
        .iter()
        .rfind(|token| token.end <= first_column_start)
        .and_then(|token| token.text.parse::<u16>().ok())
}

fn school_column_start(section: &Section, name_start: Option<usize>) -> Option<usize> {
    ["School", "Team", "Relay", "Athlete"]
        .iter()
        .find_map(|label| section.column(label).map(|column| column.start))
        .or(name_start)
}

fn school_and_name(
    line: &str,
    tokens: &[Token<'_>],
    section: &Section,
    school_start: usize,
    name_start: Option<usize>,
) -> (String, String) {
    let school = match section.next_numeric_start(tokens, school_start) {
        Some(end) => substring(line, school_start, end),
        None => substring(line, school_start, line.len()),
    };
    let name = match name_start {
        Some(start) => {
            let end = section
                .next_numeric_start(tokens, start)
                .map_or(school_start, |value| value);
            substring(line, start, end)
        }
        None => String::new(),
    };
    (school, name)
}

fn row_grade(tokens: &[Token<'_>], section: &Section) -> Option<Grade> {
    section
        .numeric_token(tokens, "Year")
        .and_then(|token| token.text.parse::<u8>().ok())
        .and_then(Grade::new)
}

fn row_identity(
    line: &str,
    tokens: &[Token<'_>],
    section: &Section,
) -> Option<(Option<u16>, String, String, Option<Grade>)> {
    let first_column_start = section.columns.first()?.start;
    let place = row_place(tokens, first_column_start);
    let name_start = section.column("Name").map(|column| column.start);
    let school_start = school_column_start(section, name_start)?;
    let (mut school, mut name) =
        school_and_name(line, tokens, section, school_start, name_start);
    let mut grade = row_grade(tokens, section);
    if name_start.is_none() {
        if let Some((athlete, row_grade, school_label)) = individual_identity(&school) {
            name = athlete;
            grade = grade.or(Some(row_grade));
            school = school_label;
        }
    }
    if school.is_empty() || school.contains(')') {
        return None;
    }
    if !name.is_empty() && !looks_like_a_name(&name) {
        return None;
    }

    Some((place, name, school, grade))
}

pub(super) fn parse_legs(trimmed: &str, filled: usize) -> Vec<RelayLeg> {
    let Ok(relay_leg) = relay_leg_regex() else {
        return Vec::new();
    };
    let mut legs = Vec::new();
    for captures in relay_leg.captures_iter(trimmed) {
        let fallback = u8::try_from(filled.saturating_add(legs.len()).saturating_add(1))
            .map_or(u8::MAX, |value| value);
        let position: u8 = captures
            .get(1)
            .and_then(|m| m.as_str().parse().ok())
            .map_or(fallback, |value| value);
        let name = captures
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .map_or(Default::default(), core::convert::identity);
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
