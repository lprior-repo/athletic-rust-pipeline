//! Classification of a raw source record: the event key, timing basis, and
//! source unit its marks are judged against.

use super::super::{ResultEvidence, SourceUnit, TimingBasis};
use super::text::normalize;
use crate::domain::evidence::Sport;
use crate::domain::marks::EventName;

pub(in super::super) fn source_event(source: &ResultEvidence) -> EventName {
    let raw = match source.sport {
        Sport::CrossCountry => xc_event_key(&source.event_name),
        Sport::TrackField => source.event_name.clone(),
    };
    match EventName::parse_retained(&raw) {
        Ok(event) => event,
        Err(_) => EventName::Unsupported(source.event_name.clone()),
    }
}

fn xc_event_key(raw: &str) -> String {
    let normalized = normalize(raw);
    let distance = match normalized
        .strip_prefix("xc ")
        .or_else(|| normalized.strip_prefix("cross country "))
    {
        Some(value) => value,
        None => normalized.as_str(),
    };
    let compact = distance
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let distance = compact.as_str();
    if let Some(number) = ["kilometers", "kilometer", "km"]
        .into_iter()
        .find_map(|unit| distance.strip_suffix(unit))
    {
        return format!("xc{number}k");
    }
    let distance = ["meters", "meter"]
        .into_iter()
        .find_map(|unit| distance.strip_suffix(unit))
        .map_or(distance, |value| value);
    match distance {
        "2000" | "2000m" | "2k" => "xc2k".to_owned(),
        "3000" | "3000m" | "3k" => "xc3k".to_owned(),
        "4000" | "4000m" | "4k" => "xc4k".to_owned(),
        "5000" | "5000m" | "5k" => "xc5k".to_owned(),
        "6000" | "6000m" | "6k" => "xc6k".to_owned(),
        "8000" | "8000m" | "8k" => "xc8k".to_owned(),
        "10000" | "10000m" | "10k" => "xc10k".to_owned(),
        "12000" | "12000m" | "12k" => "xc12k".to_owned(),
        "2mile" | "2miles" | "2mi" => "xc2mile".to_owned(),
        "3mile" | "3miles" | "3mi" => "xc3mile".to_owned(),
        "5mile" | "5miles" | "5mi" => "xc5mile".to_owned(),
        "6mile" | "6miles" | "6mi" => "xc6mile".to_owned(),
        _ => format!("xc {raw}"),
    }
}

pub(in super::super) fn timing_for(source: &ResultEvidence, event: &EventName) -> TimingBasis {
    if source.sport == Sport::CrossCountry
        || !matches!(expected_unit(event), Some(ExpectedUnit::Seconds))
    {
        TimingBasis::NotApplicable
    } else {
        timing_basis(source.timing.as_deref())
    }
}

pub(in super::super) fn source_unit_for(source: &ResultEvidence, event: &EventName) -> SourceUnit {
    if source.sport == Sport::CrossCountry {
        return SourceUnit::Seconds;
    }
    match (expected_unit(event), source.units.as_deref()) {
        (Some(ExpectedUnit::Seconds), None) => SourceUnit::Seconds,
        (Some(ExpectedUnit::Distance), _)
            if source_unit(source.units.as_deref()) == SourceUnit::Unknown =>
        {
            explicit_distance_unit(&source.mark)
        }
        (_, raw) => source_unit(raw),
    }
}

pub(super) fn explicit_distance_unit(raw: &str) -> SourceUnit {
    let mark = raw.trim();
    if mark.contains('\'')
        || mark.split_once('-').is_some_and(|(feet, _)| {
            !feet.is_empty() && feet.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return SourceUnit::FeetInches;
    }
    let unit = mark
        .find(|character: char| character.is_ascii_alphabetic())
        .and_then(|index| mark.get(index..));
    source_unit(unit)
}

fn timing_basis(raw: Option<&str>) -> TimingBasis {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.eq_ignore_ascii_case("fat") => TimingBasis::Fat,
        Some(value) if value.eq_ignore_ascii_case("hand") || value.eq_ignore_ascii_case("ht") => {
            TimingBasis::Hand
        }
        Some(value) => TimingBasis::Other(value.to_owned()),
        None => TimingBasis::Unknown,
    }
}

fn source_unit(raw: Option<&str>) -> SourceUnit {
    match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("s" | "sec" | "second" | "seconds") => SourceUnit::Seconds,
        Some("m" | "meter" | "meters") => SourceUnit::Meters,
        Some("cm" | "centimeter" | "centimeters") => SourceUnit::Centimeters,
        Some("mm" | "millimeter" | "millimeters") => SourceUnit::Millimeters,
        Some("ft" | "feet" | "foot" | "in" | "inch" | "inches") => SourceUnit::FeetInches,
        Some("pt" | "pts" | "point" | "points") => SourceUnit::Points,
        _ => SourceUnit::Unknown,
    }
}

pub(super) fn unit_matches(event: &EventName, units: &SourceUnit) -> bool {
    match expected_unit(event) {
        Some(ExpectedUnit::Seconds) => units == &SourceUnit::Seconds,
        Some(ExpectedUnit::Distance) => matches!(
            units,
            SourceUnit::Meters
                | SourceUnit::Centimeters
                | SourceUnit::Millimeters
                | SourceUnit::FeetInches
        ),
        Some(ExpectedUnit::Points) => units == &SourceUnit::Points,
        None => false,
    }
}

#[derive(Clone, Copy)]
pub(super) enum ExpectedUnit {
    Seconds,
    Distance,
    Points,
}

pub(super) fn expected_unit(event: &EventName) -> Option<ExpectedUnit> {
    match event {
        EventName::Track(_) | EventName::Relay(_) | EventName::CrossCountry(_) => {
            Some(ExpectedUnit::Seconds)
        }
        EventName::LongJump
        | EventName::TripleJump
        | EventName::HighJump
        | EventName::PoleVault
        | EventName::ShotPut
        | EventName::Discus
        | EventName::Javelin
        | EventName::Hammer
        | EventName::WeightThrow => Some(ExpectedUnit::Distance),
        EventName::Decathlon | EventName::Heptathlon | EventName::Pentathlon => {
            Some(ExpectedUnit::Points)
        }
        _ => None,
    }
}

pub(super) fn known_event(event: &EventName) -> bool {
    !matches!(
        event,
        EventName::Unsupported(_) | EventName::CrossCountryUnknown
    )
}
