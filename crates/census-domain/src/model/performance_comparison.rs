mod catalog;
mod classification;
mod compatibility;
mod labels;
mod measure;
mod tie;

pub use classification::{classify_wind, is_wind_sensitive, resolve_timing};
pub use compatibility::ComparisonPolicy;
pub use measure::{field_micrometres, mark_unit, Measure};
pub use tie::{should_replace, tie_break_preferred, MarkOrdering};

use crate::model::{
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, CatalogError, EventDisposition, EventId,
    EventKind, EventSpecification, RecruitingCatalogV1, Sport,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct KeyAdjustment {
    pub kind: crate::model::AdjustmentKind,
    pub basis: crate::model::AdjustmentBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub struct PrKey {
    pub athlete_id: crate::model::AthleteId,
    pub event_kind: EventKind,
    pub surface: SurfaceClass,
    pub wind_class: WindClass,
    pub timing: TimingClass,
    pub measure: Measure,
    pub context: Option<EventId>,
    pub specification: EventSpecification,
    pub comparison: ComparisonPolicy,
    pub adjustment: Option<KeyAdjustment>,
}

impl PrKey {
    pub fn from_performance(
        performance: &CanonicalPerformance,
        event: &CanonicalEvent,
        meet: Option<&CanonicalMeet>,
        measure: Measure,
        semantics: crate::model::EventSemantics,
    ) -> Result<Option<Self>, CatalogError> {
        Self::from_athlete(
            performance,
            event,
            meet,
            measure,
            &performance.athlete,
            semantics,
        )
    }

    fn from_athlete(
        performance: &CanonicalPerformance,
        event: &CanonicalEvent,
        meet: Option<&CanonicalMeet>,
        measure: Measure,
        athlete: &crate::model::AthleteId,
        semantics: crate::model::EventSemantics,
    ) -> Result<Option<Self>, CatalogError> {
        let surface = meet.map_or(SurfaceClass::Unresolved, SurfaceClass::resolve);
        let disposition =
            RecruitingCatalogV1::with_policy(semantics).classify(event, catalog::sport(surface))?;
        Ok(Self::from_admitted(
            performance,
            event,
            meet,
            measure,
            athlete,
            &disposition,
            semantics,
        ))
    }

    pub fn from_admitted(
        performance: &CanonicalPerformance,
        event: &CanonicalEvent,
        meet: Option<&CanonicalMeet>,
        measure: Measure,
        athlete: &crate::model::AthleteId,
        disposition: &EventDisposition,
        semantics: crate::model::EventSemantics,
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
        catalog::included_key(
            performance,
            event,
            surface,
            measure,
            athlete,
            disposition,
            semantics,
        )
    }

    fn base(
        performance: &CanonicalPerformance,
        kind: &EventKind,
        surface: SurfaceClass,
        measure: Measure,
        athlete: &crate::model::AthleteId,
    ) -> Self {
        Self {
            athlete_id: athlete.clone(),
            event_kind: kind.clone(),
            surface,
            wind_class: classify_wind(surface, kind, performance.wind_mps),
            timing: resolve_timing(&performance.mark, performance.timing),
            measure,
            context: None,
            specification: EventSpecification::default(),
            comparison: ComparisonPolicy::Standard,
            adjustment: performance
                .adjustment
                .as_ref()
                .map(|adjustment| KeyAdjustment {
                    kind: adjustment.kind,
                    basis: adjustment.basis,
                }),
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
