//! Assembly of a `PerformanceContext` from a classified source record: surface,
//! event type, equipment, wind legality, and attribution.

use super::super::{
    AttributionContext, EquipmentContext, PerformanceContext, ResultEvidence, SourceUnit,
    SurfaceContext, TimingBasis, WindLegality,
};
use super::classify::known_event;
use super::text::normalize;
use crate::domain::evidence::{ResultAttribution, Sport};
use crate::domain::marks::EventName;

pub(in super::super) fn context(
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
