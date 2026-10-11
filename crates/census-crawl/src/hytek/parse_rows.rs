use crate::result_file::{ParsedRow, RelayLeg};
use census_domain::model::{EventKind, Grade, Mark, TimingMethod};

use super::columns::{looks_like_a_name, substring, tokens, Token};
use super::identity::individual_identity;
use super::map::{parse_marks, time_is_hand};
use super::parse::{relay_leg_regex, Section};

pub(super) const MARK_LABELS: [&str; 10] = [
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

pub(super) const HEAT_START_SLOP: usize = 8;

pub(super) const MAX_HEAT_WIDTH: usize = 2;

pub(super) fn heat_token(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    text.len() == 1 && text.bytes().all(|byte| byte.is_ascii_alphabetic())
}

pub(in crate::hytek) fn starts_like_a_row(trimmed: &str) -> bool {
    trimmed
        .split_whitespace()
        .next()
        .is_some_and(|token| token.chars().all(|ch| ch.is_ascii_digit()))
}

pub(in crate::hytek) fn parse_row(
    line: &str,
    kind: &EventKind,
    section: &Section,
) -> Option<ParsedRow> {
    let tokens = tokens(line);
    let (place, name, school, grade) = row_identity(line, &tokens, section)?;
    let (mark, mark_text) = row_mark(&tokens, section, kind)?;
    let heat = section.heat(&tokens);
    let points = section.points(&tokens);
    let wind = section.wind(&tokens);
    let timing = time_is_hand(mark_text).then_some(TimingMethod::Hand);

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
    let (mut school, mut name) = school_and_name(line, tokens, section, school_start, name_start);
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

pub(in crate::hytek) fn parse_legs(trimmed: &str, filled: usize) -> Vec<RelayLeg> {
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
