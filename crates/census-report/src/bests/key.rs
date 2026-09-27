mod classification;
mod tie;

pub use classification::{classify_wind, is_wind_sensitive, resolve_timing};
pub use tie::tie_break_later;

use census_domain::model::{CanonicalMeet, CanonicalPerformance, EventId, EventKind, Sport};
use std::collections::HashMap;

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
}

impl PrKey {
    pub fn from_performance(
        performance: &CanonicalPerformance,
        kind: &EventKind,
        meet: Option<&CanonicalMeet>,
        measure: crate::bests::Measure,
    ) -> Option<Self> {
        let surface = SurfaceClass::resolve(meet?);
        if surface == SurfaceClass::Unresolved {
            return None;
        }
        let contextual = surface == SurfaceClass::CrossCountry
            || matches!(kind, EventKind::CrossCountry | EventKind::Unmapped { .. });
        Some(Self {
            athlete_id: performance.athlete.clone(),
            event_kind: kind.clone(),
            surface,
            wind_class: classify_wind(surface, kind, performance.wind_mps),
            timing: resolve_timing(&performance.mark, performance.timing),
            measure,
            context: contextual.then(|| performance.event.clone()),
        })
    }
}

impl std::fmt::Display for PrKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}-{}/{}",
            self.athlete_id,
            self.event_kind.stable_key(),
            match self.surface {
                SurfaceClass::Indoor => "indoor",
                SurfaceClass::Outdoor => "outdoor",
                SurfaceClass::CrossCountry => "xc",
                SurfaceClass::Unresolved => "unresolved",
            },
            match self.wind_class {
                WindClass::Legal => "legal",
                WindClass::Assisted => "assisted",
                WindClass::Unknown => "unknown",
                WindClass::NotApplicable => "na",
            },
            match self.timing {
                TimingClass::Fat => "fat",
                TimingClass::Hand => "hand",
                TimingClass::Unknown => "unknown",
                TimingClass::NonTime => "non_time",
            },
        )
    }
}

pub fn should_replace(
    candidate_value: i64,
    incumbent_value: i64,
    cand_date: &str,
    cand_meet: &str,
    cand_perf_id: &str,
    inc_date: &str,
    inc_meet: &str,
    inc_perf_id: &str,
    better: impl Fn(i64, i64) -> bool,
) -> bool {
    tie::should_replace_impl(
        candidate_value,
        incumbent_value,
        cand_date,
        cand_meet,
        cand_perf_id,
        inc_date,
        inc_meet,
        inc_perf_id,
        better,
    )
}
