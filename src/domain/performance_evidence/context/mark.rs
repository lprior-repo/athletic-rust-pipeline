//! Parsing of a raw mark string into a retained `Performance`, including the
//! timing-suffix and distance-unit reconciliation rules.

use super::super::{MarkObservation, ResultEvidence, SourceUnit, TimingBasis};
use super::classify::{expected_unit, explicit_distance_unit, unit_matches, ExpectedUnit};
use crate::domain::marks::{EventName, Performance};

pub(in super::super) fn parse_mark(
    source: &ResultEvidence,
    event: &EventName,
    timing: &TimingBasis,
    units: &SourceUnit,
) -> MarkObservation {
    if let Some(status) = Performance::from_status(event, &source.mark) {
        return MarkObservation::Parsed(status);
    }
    if !unit_matches(event, units) {
        return unsupported(
            &source.mark,
            "source unit is absent or incompatible with the event",
        );
    }
    if matches!(expected_unit(event), Some(ExpectedUnit::Distance)) {
        let explicit = explicit_distance_unit(&source.mark);
        if explicit != SourceUnit::Unknown && &explicit != units {
            return unsupported(&source.mark, "mark unit contradicts declared source unit");
        }
    }
    let mark = match mark_without_suffix(&source.mark, timing) {
        Ok(value) => value,
        Err(reason) => return unsupported(&source.mark, reason),
    };
    let mark = if matches!(expected_unit(event), Some(ExpectedUnit::Distance)) {
        distance_mark(mark, units, source.units.as_deref())
    } else {
        mark
    };
    match Performance::parse_retained(event, &mark) {
        Ok(value) => MarkObservation::Parsed(value),
        Err(reason) => unsupported(&source.mark, reason.to_string()),
    }
}

fn unsupported(raw: &str, reason: impl Into<String>) -> MarkObservation {
    MarkObservation::Unsupported {
        raw: raw.to_owned(),
        reason: reason.into(),
    }
}

fn mark_without_suffix(raw: &str, timing: &TimingBasis) -> Result<String, &'static str> {
    let value = raw.trim();
    let Some(last) = value.chars().last() else {
        return Ok(value.to_owned());
    };
    let suffix = last.to_ascii_lowercase();
    if !matches!(suffix, 'a' | 'h' | 'c') {
        return Ok(value.to_owned());
    }
    let corroborated = matches!(
        (suffix, timing),
        ('a', &TimingBasis::Fat) | ('h', &TimingBasis::Hand) | ('c', &TimingBasis::Hand)
    );
    if !corroborated {
        return Err("mark suffix lacks corroborating timing metadata");
    }
    let core = value.strip_suffix(last).map_or("", str::trim_end);
    if core.is_empty() {
        return Err("mark suffix has no numeric mark");
    }
    Ok(core.to_owned())
}

fn distance_mark(mut raw: String, units: &SourceUnit, declared: Option<&str>) -> String {
    if raw.contains('\'')
        || raw.contains('-')
        || raw.chars().any(|character| character.is_ascii_alphabetic())
    {
        return raw;
    }
    let suffix = match units {
        SourceUnit::Meters => "m",
        SourceUnit::Centimeters => "cm",
        SourceUnit::Millimeters => "mm",
        SourceUnit::FeetInches => match declared {
            Some(value) => value,
            None => return raw,
        },
        SourceUnit::Seconds | SourceUnit::Points | SourceUnit::Unknown => return raw,
    };
    raw.push_str(suffix);
    raw
}
