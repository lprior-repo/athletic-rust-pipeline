use super::super::evidence::{ResultAttribution, Sport};
use super::super::marks::{compare_performances, Comparison, EventName, Performance};
use super::{
    AttributionContext, BestClaim, BestClaimKind, EquipmentContext, MarkObservation,
    ObservedBestGroup, PerformanceContext, PerformanceObservation, ResultEvidence, SourceBestClaim,
    SourceUnit, SurfaceContext, TimingBasis, WindLegality,
};

pub(super) fn source_event(source: &ResultEvidence) -> EventName {
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

pub(super) fn context(
    source: &ResultEvidence,
    event: EventName,
    timing: TimingBasis,
    units: SourceUnit,
) -> PerformanceContext {
    let distance = known_event(&event).then(|| event.as_str().to_owned());
    PerformanceContext {
        sport: source.sport,
        event: event.clone(),
        distance,
        surface: surface(source),
        event_type: source
            .event_type
            .as_deref()
            .map(normalize)
            .filter(|value| !value.is_empty()),
        timing,
        units,
        equipment: equipment(&event, source.event_description.as_deref()),
        wind: wind(source, &event),
        attribution: attribution(&source.attribution),
    }
}

pub(super) fn parse_mark(
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
    let end = value.len() - last.len_utf8();
    let core = value[..end].trim_end();
    if core.is_empty() {
        return Err("mark suffix has no numeric mark");
    }
    Ok(core.to_owned())
}

pub(super) fn timing_for(source: &ResultEvidence, event: &EventName) -> TimingBasis {
    if source.sport == Sport::CrossCountry
        || !matches!(expected_unit(event), Some(ExpectedUnit::Seconds))
    {
        TimingBasis::NotApplicable
    } else {
        timing_basis(source.timing.as_deref())
    }
}

pub(super) fn source_unit_for(source: &ResultEvidence, event: &EventName) -> SourceUnit {
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

fn explicit_distance_unit(raw: &str) -> SourceUnit {
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
        .map(|index| &mark[index..]);
    source_unit(unit)
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

fn unit_matches(event: &EventName, units: &SourceUnit) -> bool {
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
enum ExpectedUnit {
    Seconds,
    Distance,
    Points,
}

fn expected_unit(event: &EventName) -> Option<ExpectedUnit> {
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

fn known_event(event: &EventName) -> bool {
    !matches!(
        event,
        EventName::Unsupported(_) | EventName::CrossCountryUnknown
    )
}

fn surface(source: &ResultEvidence) -> SurfaceContext {
    if source.sport == Sport::CrossCountry {
        return SurfaceContext::NotApplicable;
    }
    match source.season {
        1..=9_999 => SurfaceContext::Outdoor,
        10_000..=u16::MAX => SurfaceContext::Indoor,
        _ => SurfaceContext::Unknown,
    }
}

fn equipment(event: &EventName, description: Option<&str>) -> EquipmentContext {
    if event.as_str().ends_with('h') {
        return descriptor_with_digit(description, EquipmentContext::Hurdles);
    }
    if matches!(
        event,
        EventName::ShotPut
            | EventName::Discus
            | EventName::Javelin
            | EventName::Hammer
            | EventName::WeightThrow
    ) {
        return descriptor_with_digit(description, EquipmentContext::Implement);
    }
    if matches!(
        event,
        EventName::LongJump | EventName::TripleJump | EventName::HighJump | EventName::PoleVault
    ) {
        return EquipmentContext::NotApplicable;
    }
    if known_event(event) {
        EquipmentContext::NotApplicable
    } else {
        EquipmentContext::Unknown
    }
}

fn descriptor_with_digit<F>(description: Option<&str>, build: F) -> EquipmentContext
where
    F: FnOnce(String) -> EquipmentContext,
{
    let descriptor = description
        .map(normalize)
        .filter(|value| value.chars().any(|ch| ch.is_ascii_digit()));
    descriptor.map_or(EquipmentContext::Unknown, build)
}

fn wind(source: &ResultEvidence, event: &EventName) -> WindLegality {
    if source.sport == Sport::CrossCountry || source.season >= 10_000 || !wind_applicable(event) {
        return WindLegality::NotApplicable;
    }
    let Some(raw) = source
        .wind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return WindLegality::Unknown;
    };
    match raw.parse::<f64>() {
        Ok(value) if value.is_finite() && value <= 2.0 => WindLegality::Legal,
        Ok(value) if value.is_finite() => WindLegality::Illegal,
        _ => WindLegality::Unknown,
    }
}

fn wind_applicable(event: &EventName) -> bool {
    match event {
        EventName::LongJump | EventName::TripleJump => true,
        _ => matches!(
            event.as_str(),
            "55m" | "60m" | "80m" | "100m" | "200m" | "55h" | "60h" | "80h" | "100h" | "110h"
        ),
    }
}

fn attribution(value: &ResultAttribution) -> AttributionContext {
    match value {
        ResultAttribution::Individual => AttributionContext::Individual,
        ResultAttribution::VerifiedRelayMember { relay_athlete_id } => {
            AttributionContext::Relay(*relay_athlete_id)
        }
        ResultAttribution::Unresolved { .. } => AttributionContext::Unknown,
    }
}

pub(super) fn source_claims(source: &ResultEvidence) -> Vec<SourceBestClaim> {
    [
        (BestClaimKind::PersonalBest, source.personal_best.clone()),
        (BestClaimKind::SeasonBest, source.season_best.clone()),
    ]
    .into_iter()
    .map(|(kind, raw)| SourceBestClaim {
        result_id: source.result_id,
        kind: kind.clone(),
        claimed: claim_value(&raw, source.sport, &kind),
        raw,
        evidence: source.evidence.clone(),
    })
    .collect()
}

fn claim_value(claim: &BestClaim, sport: Sport, kind: &BestClaimKind) -> Option<bool> {
    match (sport, claim, kind) {
        (Sport::TrackField, BestClaim::OpaqueFlags(raw), BestClaimKind::PersonalBest) => {
            Some(raw & 2 == 2)
        }
        (Sport::TrackField, BestClaim::OpaqueFlags(raw), BestClaimKind::SeasonBest) => {
            Some(raw & 1 == 1)
        }
        (Sport::CrossCountry, BestClaim::Claimed, _) => Some(true),
        (Sport::CrossCountry, BestClaim::NotClaimed, _) => Some(false),
        _ => None,
    }
}

pub(super) fn observed_bests(observations: &[PerformanceObservation]) -> Vec<ObservedBestGroup> {
    observations
        .iter()
        .filter(|item| eligible(item))
        .fold(Vec::new(), |mut groups, observation| {
            let group = groups
                .iter_mut()
                .find(|item| comparable_context(&item.context, &observation.context));
            match group {
                Some(group) if improves(observation, &group.best) => {
                    group.best = observation.clone()
                }
                Some(_) => {}
                None => groups.push(ObservedBestGroup {
                    context: observation.context.clone(),
                    best: observation.clone(),
                }),
            }
            groups
        })
}

fn eligible(observation: &PerformanceObservation) -> bool {
    observation.result_id > 0
        && matches!(
            &observation.context.attribution,
            AttributionContext::Individual
        )
        && known_context(&observation.context)
        && matches!(&observation.mark, MarkObservation::Parsed(mark) if mark.comparable().is_some())
}

fn comparable_context(left: &PerformanceContext, right: &PerformanceContext) -> bool {
    left == right && known_context(left)
}

fn known_context(context: &PerformanceContext) -> bool {
    let timing_known = matches!(
        &context.timing,
        TimingBasis::Fat | TimingBasis::Hand | TimingBasis::NotApplicable
    );
    let wind_known = matches!(
        &context.wind,
        WindLegality::Legal | WindLegality::Illegal | WindLegality::NotApplicable
    );
    let event_type_known = context.sport == Sport::CrossCountry || context.event_type.is_some();
    let equipment_known = if context.event.as_str().ends_with('h') {
        matches!(&context.equipment, EquipmentContext::Hurdles(_))
    } else if matches!(
        context.event,
        EventName::ShotPut
            | EventName::Discus
            | EventName::Javelin
            | EventName::Hammer
            | EventName::WeightThrow
    ) {
        matches!(&context.equipment, EquipmentContext::Implement(_))
    } else {
        true
    };
    context.distance.is_some()
        && !matches!(&context.surface, SurfaceContext::Unknown)
        && timing_known
        && !matches!(&context.units, SourceUnit::Unknown)
        && wind_known
        && event_type_known
        && equipment_known
}

fn improves(candidate: &PerformanceObservation, previous: &PerformanceObservation) -> bool {
    match (&candidate.mark, &previous.mark) {
        (MarkObservation::Parsed(left), MarkObservation::Parsed(right)) => {
            compare_performances(left, right) == Comparison::Better
        }
        _ => false,
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
