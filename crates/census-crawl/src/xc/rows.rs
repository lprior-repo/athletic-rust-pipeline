use crate::hytek::{self, grade_from_token, looks_like_a_name, substring};
use crate::result_file::ParsedRow;
use census_domain::model::{Gender, Mark};
use regex::Captures;

use super::patterns::{block_row, gender_heading, grade_table_row_regex, race_banner};

pub(super) fn section_heading(trimmed: &str) -> Option<(Gender, String)> {
    let captures = gender_heading().ok()?.captures(trimmed)?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = match captures
        .get(2)
        .map(|label| label.as_str().trim().to_string())
        .filter(|label| !label.is_empty())
    {
        Some(value) => value,
        None => "Varsity".to_string(),
    };
    Some((gender, label))
}

pub(super) fn race_heading(inner: &str) -> Option<(Gender, String)> {
    let captures = race_banner().ok()?.captures(inner)?;
    let gender = match captures.get(1)?.as_str().to_ascii_lowercase().as_str() {
        "boys" | "men" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    (!label.is_empty()).then_some((gender, label))
}

pub(super) fn rule_spans(line: &str) -> Option<Vec<(usize, usize)>> {
    if !line.trim_start().starts_with('=') {
        return None;
    }
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (index, ch) in line.char_indices() {
        if ch == '=' {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(from) = start.take() {
            spans.push((from, index));
        }
    }
    if let Some(from) = start {
        spans.push((from, line.len()));
    }
    (spans.len() >= 5).then_some(spans)
}

pub(super) fn block_rows(line: &str, team: Option<&str>) -> (Vec<ParsedRow>, usize) {
    let Some(team) = team else {
        return (Vec::new(), 0);
    };
    let Ok(regex) = block_row() else {
        return (Vec::new(), 0);
    };
    regex
        .captures_iter(line)
        .fold(
            (Vec::new(), 0usize),
            |(mut rows, skipped), captures| match block_runner(&captures, team) {
                Some(row) => {
                    rows.push(row);
                    (rows, skipped)
                }
                None => (rows, skipped.saturating_add(1)),
            },
        )
}

fn block_runner(captures: &Captures<'_>, team: &str) -> Option<ParsedRow> {
    let name = captures.get(3)?.as_str().trim();
    if !looks_like_a_name(name) {
        return None;
    }
    let seconds = hytek::parse_time(captures.get(5)?.as_str())?;
    Some(ParsedRow {
        place: block_place(captures.get(2)?.as_str()),
        name: name.to_string(),
        grade: grade_from_token(captures.get(4)?.as_str()),
        school: team.to_string(),
        mark: Mark::TimeSeconds(seconds),
        timing: None,
        wind_mps: None,
        heat: None,
        points: None,
        legs: Vec::new(),
    })
}

fn block_place(raw: &str) -> Option<u16> {
    raw.trim()
        .trim_matches(|ch| ch == '(' || ch == ')')
        .trim()
        .parse()
        .ok()
}

pub(super) fn grade_table_row(line: &str) -> Option<ParsedRow> {
    let captures = grade_table_row_regex().ok()?.captures(line)?;
    let name = captures.get(4)?.as_str().trim().to_string();
    let school = captures.get(5)?.as_str().trim().to_string();
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(captures.get(8)?.as_str())?;
    Some(ParsedRow {
        place: captures.get(1)?.as_str().parse::<u16>().ok(),
        name,
        grade: grade_from_token(captures.get(7)?.as_str()),
        school,
        mark: Mark::TimeSeconds(seconds),
        timing: None,
        wind_mps: None,
        heat: None,
        points: None,
        legs: Vec::new(),
    })
}

pub(super) fn accurace_row(line: &str, spans: Option<&[(usize, usize)]>) -> Option<ParsedRow> {
    let spans = spans?;
    if spans.len() < 8 {
        return None;
    }
    let cell = |index: usize| {
        let (start, end) = spans.get(index).copied()?;
        Some(substring(line, start, end))
    };
    let name = cell(4)?;
    let school = cell(6)?;
    if !looks_like_a_name(&name) || school.is_empty() {
        return None;
    }
    let seconds = hytek::parse_time(&cell(7)?)?;
    let grade = grade_from_token(&cell(5)?);
    Some(ParsedRow {
        place: cell(0)?.parse::<u16>().ok(),
        name,
        grade,
        school,
        mark: Mark::TimeSeconds(seconds),
        timing: None,
        wind_mps: None,
        heat: None,
        points: None,
        legs: Vec::new(),
    })
}

pub(super) fn starts_like_a_row(trimmed: &str) -> bool {
    trimmed
        .split_whitespace()
        .next()
        .is_some_and(|token| token.bytes().all(|byte| byte.is_ascii_digit()))
}
