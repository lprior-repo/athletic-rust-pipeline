use super::{
    CanonicalEvent, EventKind, Sport, CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES,
    CATALOG_V1_MAX_LABEL_BYTES, CATALOG_V1_MAX_SOURCE_LABELS,
};
use crate::model::{CompetitionCategory, Gender, SpecificationError};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventDisposition {
    Included(EventKind),
    Excluded(EventExclusionReason),
    Unresolved(EventResolutionReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventExclusionReason {
    CompetitiveWalking,
    WheelchairRace,
    Handcycle,
    RoadRace,
    Marathon,
    Ultra,
    NonCrossCountryTrailOrMountain,
    Triathlon,
    UnrelatedSport,
    MultipleExcludedDisciplines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResolutionReason {
    ContradictoryDeclarations,
    AmbiguousAdaptiveOrSpecialLabel,
    UnsupportedScholasticVariant,
    UnknownOrMixedSourceLabels,
    MissingSourceOwnership,
    UnresolvedSport,
    UnresolvedCategoryOrContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("source event label count exceeds the catalog input ceiling")]
    LabelCountExceeded,
    #[error("source event label exceeds the catalog UTF-8 byte ceiling")]
    LabelBytesExceeded,
    #[error("aggregate source event labels exceed the catalog UTF-8 byte ceiling")]
    AggregateLabelBytesExceeded,
    #[error("aggregate source event label byte count overflowed")]
    AggregateLabelBytesOverflow,
    #[error("source event label is empty or contains a control character")]
    InvalidLabel,
    #[error("catalog label component buffer allocation failed")]
    ResourceExhausted,
}

pub(super) fn admit(event: &CanonicalEvent) -> Result<(), CatalogError> {
    if event.source_labels.len() > CATALOG_V1_MAX_SOURCE_LABELS {
        return Err(CatalogError::LabelCountExceeded);
    }
    event
        .source_labels
        .iter()
        .try_fold(0usize, |bytes, label| {
            admit_label(&label.label)?;
            let bytes = bytes
                .checked_add(label.label.len())
                .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
            if bytes > CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES {
                return Err(CatalogError::AggregateLabelBytesExceeded);
            }
            Ok(bytes)
        })?;
    if let EventKind::Unmapped { label } = &event.kind {
        admit_label(label)?;
    }
    Ok(())
}

fn admit_label(label: &str) -> Result<(), CatalogError> {
    if label.len() > CATALOG_V1_MAX_LABEL_BYTES {
        return Err(CatalogError::LabelBytesExceeded);
    }
    if label.trim().is_empty() || label.chars().any(char::is_control) {
        return Err(CatalogError::InvalidLabel);
    }
    Ok(())
}

pub(super) struct Declarations {
    included: Inclusion,
    excluded: Exclusion,
    unresolved: Resolution,
}

enum Inclusion {
    Absent,
    One(EventKind),
    Contradictory,
}

enum Exclusion {
    Absent,
    One(EventExclusionReason),
}

enum Resolution {
    Absent,
    One(EventResolutionReason),
}

impl Declarations {
    pub(super) const fn new() -> Self {
        Self {
            included: Inclusion::Absent,
            excluded: Exclusion::Absent,
            unresolved: Resolution::Absent,
        }
    }

    pub(super) fn add(mut self, disposition: EventDisposition) -> Self {
        match disposition {
            EventDisposition::Included(kind) => {
                self.included = match self.included {
                    Inclusion::Absent => Inclusion::One(kind),
                    Inclusion::One(old) if old == kind => Inclusion::One(old),
                    Inclusion::One(_) | Inclusion::Contradictory => Inclusion::Contradictory,
                };
            }
            EventDisposition::Excluded(reason) => {
                self.excluded = match self.excluded {
                    Exclusion::Absent => Exclusion::One(reason),
                    Exclusion::One(old) if old == reason => Exclusion::One(old),
                    Exclusion::One(_) => {
                        Exclusion::One(EventExclusionReason::MultipleExcludedDisciplines)
                    }
                };
            }
            EventDisposition::Unresolved(reason) => {
                self.unresolved = match self.unresolved {
                    Resolution::Absent => Resolution::One(reason),
                    Resolution::One(old) => Resolution::One(old.min(reason)),
                };
            }
        }
        self
    }

    pub(super) fn finish(self) -> EventDisposition {
        match (self.included, self.excluded, self.unresolved) {
            (Inclusion::Contradictory, _, _) | (Inclusion::One(_), Exclusion::One(_), _) => {
                EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations)
            }
            (_, _, Resolution::One(reason)) => EventDisposition::Unresolved(reason),
            (Inclusion::One(kind), Exclusion::Absent, Resolution::Absent) => {
                EventDisposition::Included(kind)
            }
            (Inclusion::Absent, Exclusion::One(reason), Resolution::Absent) => {
                EventDisposition::Excluded(reason)
            }
            (Inclusion::Absent, Exclusion::Absent, Resolution::Absent) => {
                EventDisposition::Unresolved(EventResolutionReason::UnknownOrMixedSourceLabels)
            }
        }
    }
}

pub(super) fn qualify(
    event: &CanonicalEvent,
    sport: Sport,
    kind: EventKind,
) -> Result<EventDisposition, CatalogError> {
    let unresolved = |reason| Ok(EventDisposition::Unresolved(reason));
    let canonical_matches = match &event.kind {
        EventKind::Unmapped { .. } => event.resolved_source_kind().as_ref() == Some(&kind),
        canonical => canonical == &kind,
    };
    if !canonical_matches || !event.source_bound_kind(&kind) {
        return unresolved(EventResolutionReason::ContradictoryDeclarations);
    }
    match sport {
        Sport::Unknown => return unresolved(EventResolutionReason::UnresolvedSport),
        Sport::CrossCountry if kind != EventKind::CrossCountry => {
            return unresolved(EventResolutionReason::ContradictoryDeclarations);
        }
        Sport::IndoorTrack | Sport::OutdoorTrack if kind == EventKind::CrossCountry => {
            return unresolved(EventResolutionReason::ContradictoryDeclarations);
        }
        Sport::CrossCountry | Sport::IndoorTrack | Sport::OutdoorTrack => {}
    }
    let specification = match event.resolved_specification() {
        Ok(value) => value,
        Err(SpecificationError::ConflictingSpecification) => {
            return unresolved(EventResolutionReason::ContradictoryDeclarations);
        }
        Err(_) => return unresolved(EventResolutionReason::UnresolvedCategoryOrContext),
    };
    match category(specification.category.as_ref(), event.gender) {
        Ok(()) => Ok(EventDisposition::Included(kind)),
        Err(reason) => unresolved(reason),
    }
}

fn category(
    category: Option<&CompetitionCategory>,
    gender: Gender,
) -> Result<(), EventResolutionReason> {
    let explicit = match category {
        Some(CompetitionCategory::Boys) => Some(Gender::Boys),
        Some(CompetitionCategory::Girls) => Some(Gender::Girls),
        Some(CompetitionCategory::Mixed) => Some(Gender::Mixed),
        Some(CompetitionCategory::Published(label)) => {
            match super::super::event_specification::published_category_gender(label) {
                Some(CompetitionCategory::Boys) => Some(Gender::Boys),
                Some(CompetitionCategory::Girls) => Some(Gender::Girls),
                Some(CompetitionCategory::Mixed) => Some(Gender::Mixed),
                _ => return Err(EventResolutionReason::UnresolvedCategoryOrContext),
            }
        }
        None => None,
    };
    match (explicit, gender) {
        (Some(explicit), gender) if gender != Gender::Unknown && explicit != gender => {
            Err(EventResolutionReason::ContradictoryDeclarations)
        }
        (None, Gender::Unknown) => Err(EventResolutionReason::UnresolvedCategoryOrContext),
        _ => Ok(()),
    }
}
