use super::{CentiPoints, EventKind, HurdleSpecification, ImplementMass};
use serde::{Deserialize, Serialize};

pub const MAX_COMBINED_COMPONENTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "DisciplineWire")]
pub struct CombinedDiscipline {
    kind: EventKind,
    hurdles: Option<HurdleSpecification>,
    implement: Option<ImplementMass>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DisciplineWire {
    kind: EventKind,
    hurdles: Option<HurdleSpecification>,
    implement: Option<ImplementMass>,
}

impl CombinedDiscipline {
    pub fn new(
        kind: EventKind,
        hurdles: Option<HurdleSpecification>,
        implement: Option<ImplementMass>,
    ) -> Result<Self, CombinedEventError> {
        if kind.is_relay()
            || matches!(
                kind,
                EventKind::CrossCountry
                    | EventKind::Pentathlon
                    | EventKind::Heptathlon
                    | EventKind::Decathlon
                    | EventKind::Unmapped { .. }
            )
            || hurdles.is_some() && !kind.is_hurdles()
            || implement.is_some()
                && !matches!(
                    kind,
                    EventKind::ShotPut
                        | EventKind::Discus
                        | EventKind::Javelin
                        | EventKind::Hammer
                        | EventKind::WeightThrow
                )
        {
            return Err(CombinedEventError::InvalidDiscipline);
        }
        if let Some(value) = hurdles {
            HurdleSpecification::new(value.height_micrometres, value.spacing_micrometres)
                .map_err(|_| CombinedEventError::InvalidDiscipline)?;
        }
        Ok(Self {
            kind,
            hurdles,
            implement,
        })
    }

    pub fn kind(&self) -> &EventKind {
        &self.kind
    }
}

impl TryFrom<DisciplineWire> for CombinedDiscipline {
    type Error = CombinedEventError;

    fn try_from(value: DisciplineWire) -> Result<Self, Self::Error> {
        Self::new(value.kind, value.hurdles, value.implement)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "SpecificationWire")]
pub struct CombinedEventSpecification {
    kind: EventKind,
    lineup: Vec<CombinedDiscipline>,
    scoring_name: String,
    scoring_revision: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecificationWire {
    kind: EventKind,
    #[serde(deserialize_with = "lineup_wire")]
    lineup: Vec<CombinedDiscipline>,
    scoring_name: String,
    scoring_revision: u32,
}

impl CombinedEventSpecification {
    pub fn new(
        kind: EventKind,
        lineup: Vec<CombinedDiscipline>,
        scoring_name: String,
        scoring_revision: u32,
    ) -> Result<Self, CombinedEventError> {
        let expected = component_count(&kind).ok_or(CombinedEventError::InvalidKind)?;
        if lineup.len() != expected {
            return Err(CombinedEventError::WrongComponentCount {
                expected,
                actual: lineup.len(),
            });
        }
        if scoring_revision == 0
            || scoring_name.trim().is_empty()
            || scoring_name.len() > 128
            || scoring_name.chars().any(char::is_control)
        {
            return Err(CombinedEventError::InvalidScoringRule);
        }
        Ok(Self {
            kind,
            lineup,
            scoring_name,
            scoring_revision,
        })
    }

    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    pub fn lineup(&self) -> &[CombinedDiscipline] {
        &self.lineup
    }

    pub fn scoring_name(&self) -> &str {
        &self.scoring_name
    }

    pub const fn scoring_revision(&self) -> u32 {
        self.scoring_revision
    }
}

impl TryFrom<SpecificationWire> for CombinedEventSpecification {
    type Error = CombinedEventError;

    fn try_from(value: SpecificationWire) -> Result<Self, Self::Error> {
        Self::new(
            value.kind,
            value.lineup,
            value.scoring_name,
            value.scoring_revision,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombinedComponentResult {
    Completed(CentiPoints),
    DidNotStart,
    DidNotFinish,
    Disqualified,
    NoMark,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CompletionWire")]
pub struct CombinedEventCompletion {
    specification: CombinedEventSpecification,
    components: Vec<CombinedComponentResult>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionWire {
    specification: CombinedEventSpecification,
    #[serde(deserialize_with = "components_wire")]
    components: Vec<CombinedComponentResult>,
}

impl CombinedEventCompletion {
    pub fn new(
        specification: CombinedEventSpecification,
        components: Vec<CombinedComponentResult>,
    ) -> Result<Self, CombinedEventError> {
        let expected = specification.lineup().len();
        if components.len() != expected {
            return Err(CombinedEventError::WrongComponentCount {
                expected,
                actual: components.len(),
            });
        }
        if components.iter().take(MAX_COMBINED_COMPONENTS).any(|component| matches!(component, CombinedComponentResult::Completed(points) if points.value() < 0)) {
            return Err(CombinedEventError::InvalidPoints);
        }
        Ok(Self {
            specification,
            components,
        })
    }

    pub fn specification(&self) -> &CombinedEventSpecification {
        &self.specification
    }

    pub fn components(&self) -> &[CombinedComponentResult] {
        &self.components
    }

    pub fn completed_total(&self) -> Result<Option<CentiPoints>, CombinedEventError> {
        let mut total = 0_i32;
        for component in self.components.iter().take(MAX_COMBINED_COMPONENTS) {
            let CombinedComponentResult::Completed(points) = component else {
                return Ok(None);
            };
            total = total
                .checked_add(points.value())
                .ok_or(CombinedEventError::PointsOverflow)?;
        }
        Ok(Some(CentiPoints::new(total)))
    }
}

impl TryFrom<CompletionWire> for CombinedEventCompletion {
    type Error = CombinedEventError;

    fn try_from(value: CompletionWire) -> Result<Self, Self::Error> {
        Self::new(value.specification, value.components)
    }
}

fn component_count(kind: &EventKind) -> Option<usize> {
    match kind {
        EventKind::Pentathlon => Some(5),
        EventKind::Heptathlon => Some(7),
        EventKind::Decathlon => Some(MAX_COMBINED_COMPONENTS),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CombinedEventError {
    #[error("event kind is not a combined event")]
    InvalidKind,
    #[error("combined-event discipline or technical dimensions are invalid")]
    InvalidDiscipline,
    #[error("combined event needs {expected} components, received {actual}")]
    WrongComponentCount { expected: usize, actual: usize },
    #[error("scoring rule name or revision is absent or invalid")]
    InvalidScoringRule,
    #[error("component points must not be negative")]
    InvalidPoints,
    #[error("combined-event point total exceeds the supported bound")]
    PointsOverflow,
}

#[cfg(test)]
#[path = "combined_event/tests.rs"]
mod tests;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombinedEligibility {
    Eligible,
    MissingSpecification,
    MissingCompletion,
    SpecificationConflict,
    Incomplete,
    TotalMismatch,
    PointsOverflow,
}

fn lineup_wire<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CombinedDiscipline>, D::Error> {
    super::serde_bounds::sequence::<D, CombinedDiscipline, MAX_COMBINED_COMPONENTS>(deserializer)
}

fn components_wire<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CombinedComponentResult>, D::Error> {
    super::serde_bounds::sequence::<D, CombinedComponentResult, MAX_COMBINED_COMPONENTS>(
        deserializer,
    )
}
