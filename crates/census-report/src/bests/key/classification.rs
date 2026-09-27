use census_domain::model::{EventKind, Mark, TimingMethod};

use super::super::Measure;
use super::{PrKey, SurfaceClass, TimingClass, WindClass};

pub const fn is_wind_sensitive(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Track100m
            | EventKind::Track200m
            | EventKind::Track100mHurdles
            | EventKind::Track110mHurdles
            | EventKind::LongJump
            | EventKind::TripleJump
    )
}

pub fn classify_wind(surface: SurfaceClass, kind: &EventKind, wind_mps: Option<f64>) -> WindClass {
    if surface != SurfaceClass::Outdoor || !is_wind_sensitive(kind) {
        return WindClass::NotApplicable;
    }

    match wind_mps {
        Some(w) if w.is_finite() && w <= 2.0 => WindClass::Legal,
        Some(w) if w.is_finite() && w > 2.0 => WindClass::Assisted,
        _ => WindClass::Unknown,
    }
}

pub fn resolve_timing(mark: &Mark, timing: Option<TimingMethod>) -> TimingClass {
    match mark {
        Mark::TimeSeconds(_) => match timing {
            Some(TimingMethod::Fat) => TimingClass::Fat,
            Some(TimingMethod::Hand) => TimingClass::Hand,
            _ => TimingClass::Unknown,
        },
        _ => TimingClass::NonTime,
    }
}
