use super::super::*;
use super::quantity;
use crate::model::EventKind;

const DISTANCE_UNITS: &[(&str, DistanceUnit, u64)] = &[
    ("km", DistanceUnit::Metres, 1_000_000),
    ("k", DistanceUnit::Metres, 1_000_000),
    ("mi", DistanceUnit::Miles, 1000),
    ("m", DistanceUnit::Metres, 1000),
];

pub(super) fn parse(
    tokens: &[&str],
    kind: &EventKind,
) -> Result<Option<CrossCountryContext>, SpecificationError> {
    if *kind != EventKind::CrossCountry {
        return Ok(None);
    }
    let distance = tokens
        .iter()
        .enumerate()
        .try_fold(None, |old, (index, token)| {
            match distance_at(tokens, index, token)? {
                Some(value) if old.is_some_and(|old| old != value) => {
                    Err(SpecificationError::ConflictingSpecification)
                }
                Some(value) => Ok(Some(value)),
                None => Ok(old),
            }
        })?;
    let Some(distance) = distance else {
        return Ok(None);
    };
    Ok(Some(CrossCountryContext {
        distance,
        course: None,
        measurement: measurement(tokens)?,
        conditions: None,
    }))
}

fn measurement(tokens: &[&str]) -> Result<CourseMeasurement, SpecificationError> {
    let measured = tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("measured"));
    let short = tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("short"));
    match (measured, short) {
        (true, false) => Ok(CourseMeasurement::PublishedMeasured),
        (false, true) => Ok(CourseMeasurement::PublishedShort),
        (false, false) => Ok(CourseMeasurement::Unknown),
        (true, true) => Err(SpecificationError::ConflictingSpecification),
    }
}

fn distance_at(
    tokens: &[&str],
    index: usize,
    token: &str,
) -> Result<Option<PublishedDistance>, SpecificationError> {
    if let Some(value) = distance_token(token)? {
        return Ok(Some(value));
    }
    let unit = if token.eq_ignore_ascii_case("mile") || token.eq_ignore_ascii_case("miles") {
        Some((DistanceUnit::Miles, 1000))
    } else if token.eq_ignore_ascii_case("km") {
        Some((DistanceUnit::Metres, 1_000_000))
    } else {
        None
    };
    let number = index.checked_sub(1).and_then(|index| tokens.get(index));
    match (number, unit) {
        (Some(number), Some((unit, multiplier))) => {
            make_distance(number, unit, multiplier).map(Some)
        }
        _ => Ok(None),
    }
}

fn distance_token(token: &str) -> Result<Option<PublishedDistance>, SpecificationError> {
    let quantity = DISTANCE_UNITS
        .iter()
        .find_map(|(suffix, unit, multiplier)| {
            let split = token.len().checked_sub(suffix.len())?;
            let number = token.get(..split)?;
            (token.get(split..)?.eq_ignore_ascii_case(suffix)
                && number.bytes().next().is_some_and(|ch| ch.is_ascii_digit()))
            .then_some((number, *unit, *multiplier))
        });
    quantity
        .map(|(number, unit, multiplier)| make_distance(number, unit, multiplier))
        .transpose()
}

fn make_distance(
    number: &str,
    unit: DistanceUnit,
    multiplier: u64,
) -> Result<PublishedDistance, SpecificationError> {
    let value = u32::try_from(quantity::scaled(number, multiplier)?)
        .map_err(|_| SpecificationError::InvalidDistance)?;
    PublishedDistance::new(unit, value)
}
