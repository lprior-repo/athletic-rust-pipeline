use super::{PrKey, SurfaceClass};
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalPerformance, EventDisposition, EventKind, Sport,
};

pub(super) const fn sport(surface: SurfaceClass) -> Sport {
    match surface {
        SurfaceClass::Indoor => Sport::IndoorTrack,
        SurfaceClass::Outdoor => Sport::OutdoorTrack,
        SurfaceClass::CrossCountry => Sport::CrossCountry,
        SurfaceClass::Unresolved => Sport::Unknown,
    }
}

pub(super) fn included_key(
    performance: &CanonicalPerformance,
    event: &CanonicalEvent,
    surface: SurfaceClass,
    measure: crate::bests::Measure,
    athlete: &AthleteId,
    disposition: &EventDisposition,
    semantics: census_domain::model::EventSemantics,
) -> Option<PrKey> {
    match disposition {
        EventDisposition::Included(kind) if !matches!(kind, EventKind::Unmapped { .. }) => {
            PrKey::base(performance, kind, surface, measure, athlete)
                .qualify_with_policy(event, semantics)
        }
        EventDisposition::Included(_)
        | EventDisposition::Excluded(_)
        | EventDisposition::Unresolved(_) => None,
    }
}
