use super::*;

#[derive(Deserialize)]
pub(super) struct HurdleWire {
    height_micrometres: u32,
    spacing_micrometres: Option<u64>,
}

impl HurdleSpecification {
    pub fn new(
        height_micrometres: u32,
        spacing_micrometres: Option<u64>,
    ) -> Result<Self, SpecificationError> {
        if height_micrometres == 0
            || height_micrometres > 2_000_000
            || spacing_micrometres.is_some_and(|spacing| spacing == 0 || spacing > 100_000_000)
        {
            return Err(SpecificationError::InvalidHurdles);
        }
        Ok(Self {
            height_micrometres,
            spacing_micrometres,
        })
    }
}

impl TryFrom<HurdleWire> for HurdleSpecification {
    type Error = SpecificationError;
    fn try_from(value: HurdleWire) -> Result<Self, Self::Error> {
        Self::new(value.height_micrometres, value.spacing_micrometres)
    }
}

#[derive(Deserialize)]
pub(super) struct TrackWire {
    length_micrometres: u64,
    banking: TrackBanking,
}

impl IndoorTrackSpecification {
    pub fn new(length_micrometres: u64, banking: TrackBanking) -> Result<Self, SpecificationError> {
        if length_micrometres == 0 || length_micrometres > 1_000_000_000 {
            return Err(SpecificationError::InvalidTrack);
        }
        Ok(Self {
            length_micrometres,
            banking,
        })
    }
}

impl TryFrom<TrackWire> for IndoorTrackSpecification {
    type Error = SpecificationError;
    fn try_from(value: TrackWire) -> Result<Self, Self::Error> {
        Self::new(value.length_micrometres, value.banking)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CategoryWire {
    Boys,
    Girls,
    Mixed,
    Published(String),
}

impl TryFrom<CategoryWire> for CompetitionCategory {
    type Error = SpecificationError;
    fn try_from(value: CategoryWire) -> Result<Self, Self::Error> {
        let category = match value {
            CategoryWire::Boys => Self::Boys,
            CategoryWire::Girls => Self::Girls,
            CategoryWire::Mixed => Self::Mixed,
            CategoryWire::Published(label) => Self::Published(label),
        };
        category.validate()?;
        Ok(category)
    }
}

impl From<CompetitionCategory> for CategoryWire {
    fn from(value: CompetitionCategory) -> Self {
        match value {
            CompetitionCategory::Boys => Self::Boys,
            CompetitionCategory::Girls => Self::Girls,
            CompetitionCategory::Mixed => Self::Mixed,
            CompetitionCategory::Published(label) => Self::Published(label),
        }
    }
}

impl CompetitionCategory {
    pub fn validate(&self) -> Result<(), SpecificationError> {
        if let Self::Published(value) = self {
            if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(SpecificationError::InvalidCategory);
            }
        }
        Ok(())
    }
}
