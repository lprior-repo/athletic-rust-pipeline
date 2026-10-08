mod classification;
mod compatibility;
mod tie;

pub use classification::{classify_wind, is_wind_sensitive, resolve_timing};
pub use compatibility::ComparisonPolicy;
pub use tie::{tie_break_later, MarkOrdering};

use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventId, EventKind, EventSpecification,
    Sport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum SurfaceClass {
    Indoor,
    Outdoor,
    CrossCountry,
    Unresolved,
}

impl SurfaceClass {
    pub fn resolve(meet: &CanonicalMeet) -> Self {
        let has_indoor = meet.sports.iter().any(|s| matches!(s, Sport::IndoorTrack));
        let has_outdoor = meet.sports.iter().any(|s| matches!(s, Sport::OutdoorTrack));
        let has_xc = meet.sports.iter().any(|s| matches!(s, Sport::CrossCountry));

        match (has_indoor, has_outdoor, has_xc) {
            (true, false, false) => Self::Indoor,
            (false, true, false) => Self::Outdoor,
            (false, false, true) => Self::CrossCountry,
            _ => Self::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum WindClass {
    Legal,
    Assisted,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum TimingClass {
    Fat,
    Hand,
    Unknown,
    NonTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub struct PrKey {
    pub athlete_id: census_domain::model::AthleteId,
    pub event_kind: EventKind,
    pub surface: SurfaceClass,
    pub wind_class: WindClass,
    pub timing: TimingClass,
    pub measure: crate::bests::Measure,
    pub context: Option<EventId>,
    pub specification: EventSpecification,
    pub comparison: ComparisonPolicy,
}

impl PrKey {
    pub fn from_performance(
        performance: &CanonicalPerformance,
        event: &CanonicalEvent,
        meet: Option<&CanonicalMeet>,
        measure: crate::bests::Measure,
    ) -> Option<Self> {
        Self::from_athlete(performance, event, meet, measure, &performance.athlete)
    }

    pub(super) fn from_athlete(
        performance: &CanonicalPerformance,
        event: &CanonicalEvent,
        meet: Option<&CanonicalMeet>,
        measure: crate::bests::Measure,
        athlete: &census_domain::model::AthleteId,
    ) -> Option<Self> {
        let meet = meet?;
        let surface = SurfaceClass::resolve(meet);
        if surface == SurfaceClass::Unresolved
            || performance.event != event.id
            || performance.meet != event.meet
            || meet.id != event.meet
        {
            return None;
        }
        let resolved = event.resolved_source_kind();
        let kind = resolved
            .as_ref()
            .map_or(&event.kind, core::convert::identity);
        Self::base(performance, kind, surface, measure, athlete).qualify(event)
    }

    fn base(
        performance: &CanonicalPerformance,
        kind: &EventKind,
        surface: SurfaceClass,
        measure: crate::bests::Measure,
        athlete: &census_domain::model::AthleteId,
    ) -> Self {
        let contextual = matches!(kind, EventKind::Unmapped { .. });
        Self {
            athlete_id: athlete.clone(),
            event_kind: kind.clone(),
            surface,
            wind_class: classify_wind(surface, kind, performance.wind_mps),
            timing: resolve_timing(&performance.mark, performance.timing),
            measure,
            context: contextual.then(|| performance.event.clone()),
            specification: EventSpecification::default(),
            comparison: ComparisonPolicy::Standard,
        }
    }
}

impl std::fmt::Display for PrKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}-{}/{}/{:?}/{:?}",
            self.athlete_id,
            self.event_kind.stable_key(),
            self.surface.label(),
            self.wind_class.label(),
            self.timing.label(),
            self.comparison,
            self.specification,
        )
    }
}

pub fn should_replace(
    candidate_value: i64,
    incumbent_value: i64,
    candidate: MarkOrdering<'_>,
    incumbent: MarkOrdering<'_>,
    better: impl Fn(i64, i64) -> bool,
) -> bool {
    tie::should_replace_impl(
        candidate_value,
        incumbent_value,
        candidate,
        incumbent,
        better,
    )
}

#[cfg(test)]
mod cen8_9_10;
