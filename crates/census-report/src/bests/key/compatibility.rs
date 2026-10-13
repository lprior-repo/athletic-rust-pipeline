use super::*;
use census_domain::model::{AdjustmentBasis, CourseMeasurement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonPolicy {
    Standard,
    ObservedFastest,
    SameCourse,
    UnqualifiedAdjustment,
}

impl ComparisonPolicy {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::ObservedFastest => "observed_fastest",
            Self::SameCourse => "same_course",
            Self::UnqualifiedAdjustment => "unqualified_adjustment",
        }
    }
}

impl PrKey {
    pub(super) fn qualify(mut self, event: &CanonicalEvent) -> Option<Self> {
        self.specification = event.resolved_specification().ok()?;
        if !self.compatible_fields() {
            return None;
        }
        if self
            .adjustment
            .is_some_and(|adjustment| adjustment.basis == AdjustmentBasis::Unknown)
            && self.comparison == ComparisonPolicy::Standard
        {
            self.comparison = ComparisonPolicy::UnqualifiedAdjustment;
        }
        if self.surface != SurfaceClass::CrossCountry {
            if self.specification.cross_country.is_some() {
                return None;
            }
            return Some(self);
        }
        self.qualify_course()
    }

    fn qualify_course(mut self) -> Option<Self> {
        self.event_kind = EventKind::CrossCountry;
        let context = self.specification.cross_country.as_mut()?;
        context.course.as_ref()?;
        match context.measurement {
            CourseMeasurement::PublishedMeasured => {
                context.course = None;
                self.comparison = ComparisonPolicy::ObservedFastest;
            }
            CourseMeasurement::PublishedShort => {
                self.comparison = ComparisonPolicy::SameCourse;
            }
            CourseMeasurement::Unknown => return None,
        }
        Some(self)
    }

    pub fn same_course(&self, event: &CanonicalEvent) -> Option<Self> {
        if self.surface != SurfaceClass::CrossCountry
            || self.comparison == ComparisonPolicy::SameCourse
        {
            return None;
        }
        let specification = event.resolved_specification().ok()?;
        specification.cross_country.as_ref()?.course.as_ref()?;
        Some(self.with_specification(specification, ComparisonPolicy::SameCourse))
    }

    fn with_specification(
        &self,
        specification: EventSpecification,
        comparison: ComparisonPolicy,
    ) -> Self {
        Self {
            athlete_id: self.athlete_id.clone(),
            event_kind: EventKind::CrossCountry,
            surface: self.surface,
            wind_class: self.wind_class,
            timing: self.timing,
            measure: self.measure,
            context: None,
            specification,
            comparison,
            adjustment: self.adjustment,
        }
    }

    fn compatible_fields(&self) -> bool {
        let implement_event = matches!(
            self.event_kind,
            EventKind::ShotPut
                | EventKind::Discus
                | EventKind::Javelin
                | EventKind::Hammer
                | EventKind::WeightThrow
        );
        (self.specification.implement.is_none() || implement_event)
            && (self.specification.hurdles.is_none() || self.event_kind.is_hurdles())
            && (self.specification.indoor_track.is_none() || self.surface == SurfaceClass::Indoor)
    }
}
