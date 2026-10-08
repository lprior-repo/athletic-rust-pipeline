use super::*;
use crate::model::{CanonicalEvent, EvidenceMethod, SourceEventLabel};

impl CanonicalEvent {
    pub fn resolved_specification(&self) -> Result<EventSpecification, SpecificationError> {
        self.specification.validate()?;
        if !self.retained_conflicts.is_empty() {
            return Err(SpecificationError::ConflictingSpecification);
        }
        let resolved_kind = self.resolved_source_kind();
        let kind = resolved_kind
            .as_ref()
            .map_or(&self.kind, core::convert::identity);
        let mut resolved = self
            .source_labels
            .iter()
            .filter(|label| self.trusted_specification_label(label))
            .try_fold(self.specification.clone(), |resolved, label| {
                resolved.merge(EventSpecification::from_published_label(
                    &label.label,
                    kind,
                )?)
            })?;
        resolved.category = resolved.category_in_context(self.gender, self.division.as_deref())?;
        Ok(resolved)
    }

    fn trusted_specification_label(&self, label: &SourceEventLabel) -> bool {
        self.evidence.iter().any(|evidence| {
            evidence.method == EvidenceMethod::Parsed
                && evidence.source == label.source
                && evidence
                    .source
                    .url
                    .as_ref()
                    .is_some_and(|url| !url.is_empty())
        })
    }
}

impl EventSpecification {
    pub fn validate(&self) -> Result<(), SpecificationError> {
        if let Some(value) = self.hurdles {
            HurdleSpecification::new(value.height_micrometres, value.spacing_micrometres)?;
        }
        if let Some(value) = self.indoor_track {
            IndoorTrackSpecification::new(value.length_micrometres, value.banking)?;
        }
        if let Some(value) = &self.category {
            value.validate()?;
        }
        Ok(())
    }

    pub fn merge(self, parsed: Self) -> Result<Self, SpecificationError> {
        Ok(Self {
            implement: merge(self.implement, parsed.implement)?,
            hurdles: merge(self.hurdles, parsed.hurdles)?,
            indoor_track: merge(self.indoor_track, parsed.indoor_track)?,
            category: merge(self.category, parsed.category)?,
            cross_country: merge_course(self.cross_country, parsed.cross_country)?,
        })
    }
}

fn merge<T: Eq>(typed: Option<T>, parsed: Option<T>) -> Result<Option<T>, SpecificationError> {
    match (typed, parsed) {
        (Some(typed), Some(parsed)) if typed != parsed => {
            Err(SpecificationError::ConflictingSpecification)
        }
        (Some(typed), _) => Ok(Some(typed)),
        (None, parsed) => Ok(parsed),
    }
}

fn merge_course(
    typed: Option<CrossCountryContext>,
    parsed: Option<CrossCountryContext>,
) -> Result<Option<CrossCountryContext>, SpecificationError> {
    match (typed, parsed) {
        (Some(typed), Some(parsed)) => {
            if typed.distance != parsed.distance
                || (parsed.measurement != CourseMeasurement::Unknown
                    && typed.measurement != parsed.measurement)
            {
                return Err(SpecificationError::ConflictingSpecification);
            }
            Ok(Some(typed))
        }
        (Some(typed), None) => Ok(Some(typed)),
        (None, parsed) => Ok(parsed),
    }
}
