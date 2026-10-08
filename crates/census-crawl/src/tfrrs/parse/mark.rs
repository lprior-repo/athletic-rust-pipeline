use super::html::{attribute, collapse_whitespace, decode_entities, text_runs};

use census_domain::model::ExactSeconds;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedMark {
    Time(String),
    Field(String),
}

pub(super) fn wind_mps(text: String) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    value.is_finite().then_some(value)
}

pub(super) fn published_mark(cell: &str) -> Option<ParsedMark> {
    let token = text_runs(cell).find(|run| reads_as_mark(run))?;
    if clock_seconds(&token).is_some() {
        Some(ParsedMark::Time(token))
    } else {
        Some(ParsedMark::Field(token))
    }
}

pub(super) fn converted_metres(cell: &str) -> Option<f64> {
    text_runs(cell).find_map(|run| metric_metres(&run))
}

pub(super) fn conversion_note(cell: &str) -> Option<String> {
    let title = attribute(cell, "title")?;
    let note = collapse_whitespace(&decode_entities(title));
    note.starts_with("Converted").then_some(note)
}

fn reads_as_mark(token: &str) -> bool {
    clock_seconds(token).is_some()
        || feet_inches_metres(token).is_some()
        || metric_metres(token).is_some()
}
pub fn clock_seconds(token: &str) -> Option<ExactSeconds> {
    crate::hytek::parse_time(token)
}

pub fn metric_metres(token: &str) -> Option<f64> {
    let digits = token.trim().strip_suffix(['m', 'M'])?;
    let metres: f64 = digits.trim().parse().ok()?;
    (metres.is_finite() && metres > 0.0).then_some(metres)
}

pub fn feet_inches_metres(token: &str) -> Option<f64> {
    let trimmed = token.trim();
    let (feet_text, inches_text) = match trimmed.split_once('\'') {
        Some((feet, rest)) => (feet, rest.trim().trim_end_matches('"').trim()),
        None => {
            let (feet, inches) = trimmed.split_once('-')?;
            (feet, inches.trim())
        }
    };
    let feet: f64 = feet_text.trim().parse().ok()?;
    let inches: f64 = if inches_text.is_empty() {
        0.0
    } else {
        inches_text.parse().ok()?
    };
    let metres = (feet * 12.0 + inches) * 0.0254;
    (metres.is_finite() && metres > 0.0).then_some(metres)
}
