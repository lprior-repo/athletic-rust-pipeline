use serde::{Deserialize, Serialize};

mod category_context;
mod labels;
mod resolve;
mod validation;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct EventSpecification {
    pub cross_country: Option<CrossCountryContext>,
    pub hurdles: Option<HurdleSpecification>,
    pub implement: Option<ImplementMass>,
    pub indoor_track: Option<IndoorTrackSpecification>,
    pub category: Option<CompetitionCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CrossCountryContext {
    pub distance: PublishedDistance,
    pub course: Option<CourseIdentity>,
    pub measurement: CourseMeasurement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<PublishedCourseConditions>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseMeasurement {
    PublishedMeasured,
    PublishedShort,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PublishedCourseConditions(String);

impl PublishedCourseConditions {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PublishedCourseConditions {
    type Error = SpecificationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(SpecificationError::InvalidCourse);
        }
        Ok(Self(value))
    }
}

impl From<PublishedCourseConditions> for String {
    fn from(value: PublishedCourseConditions) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "(String, String)", into = "(String, String)")]
pub struct CourseIdentity {
    id: String,
    version: String,
}

impl CourseIdentity {
    pub fn new(id: &str, version: &str) -> Result<Self, SpecificationError> {
        if [id, version].iter().any(|value| {
            value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control)
        }) {
            return Err(SpecificationError::InvalidCourse);
        }
        Ok(Self {
            id: id.to_string(),
            version: version.to_string(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl TryFrom<(String, String)> for CourseIdentity {
    type Error = SpecificationError;
    fn try_from(value: (String, String)) -> Result<Self, Self::Error> {
        Self::new(&value.0, &value.1)
    }
}

impl From<CourseIdentity> for (String, String) {
    fn from(value: CourseIdentity) -> Self {
        (value.id, value.version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceUnit {
    Metres,
    Miles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "DistanceWire")]
pub struct PublishedDistance {
    pub unit: DistanceUnit,
    thousandths: u32,
}

#[derive(Deserialize)]
struct DistanceWire {
    unit: DistanceUnit,
    thousandths: u32,
}

impl PublishedDistance {
    pub fn new(unit: DistanceUnit, thousandths: u32) -> Result<Self, SpecificationError> {
        if thousandths == 0 || thousandths > 100_000_000 {
            return Err(SpecificationError::InvalidDistance);
        }
        Ok(Self { unit, thousandths })
    }
    pub fn thousandths(self) -> u32 {
        self.thousandths
    }
}

impl TryFrom<DistanceWire> for PublishedDistance {
    type Error = SpecificationError;
    fn try_from(value: DistanceWire) -> Result<Self, Self::Error> {
        Self::new(value.unit, value.thousandths)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct ImplementMass(u64);

impl ImplementMass {
    pub fn micrograms(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ImplementMass {
    type Error = SpecificationError;
    fn try_from(micrograms: u64) -> Result<Self, Self::Error> {
        if micrograms == 0 || micrograms > 100_000_000_000 {
            return Err(SpecificationError::InvalidMass);
        }
        Ok(Self(micrograms))
    }
}

impl From<ImplementMass> for u64 {
    fn from(value: ImplementMass) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "validation::HurdleWire")]
pub struct HurdleSpecification {
    pub height_micrometres: u32,
    pub spacing_micrometres: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "validation::TrackWire")]
pub struct IndoorTrackSpecification {
    pub length_micrometres: u64,
    pub banking: TrackBanking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackBanking {
    Flat,
    Banked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(
    try_from = "validation::CategoryWire",
    into = "validation::CategoryWire"
)]
pub enum CompetitionCategory {
    Boys,
    Girls,
    Mixed,
    Published(String),
}

pub(super) fn published_category_label(
    category: Option<CompetitionCategory>,
    group: &str,
) -> CompetitionCategory {
    let group = group.to_ascii_lowercase();
    let prefix = match category {
        Some(CompetitionCategory::Boys) => "boys".to_owned(),
        Some(CompetitionCategory::Girls) => "girls".to_owned(),
        Some(CompetitionCategory::Mixed) => "mixed".to_owned(),
        Some(CompetitionCategory::Published(label)) => label,
        None => return CompetitionCategory::Published(group),
    };
    CompetitionCategory::Published(format!("{prefix}/{group}"))
}

pub(super) fn published_category_gender(label: &str) -> Option<CompetitionCategory> {
    match label.split('/').next()? {
        "boys" => Some(CompetitionCategory::Boys),
        "girls" => Some(CompetitionCategory::Girls),
        "mixed" => Some(CompetitionCategory::Mixed),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SpecificationError {
    #[error("published distance is absent or exceeds the supported bound")]
    InvalidDistance,
    #[error("course identity/version is absent, invalid or exceeds its bound")]
    InvalidCourse,
    #[error("published implement mass is absent or exceeds its bound")]
    InvalidMass,
    #[error("published hurdle dimensions are absent or exceed their bounds")]
    InvalidHurdles,
    #[error("published indoor track length is absent or exceeds its bound")]
    InvalidTrack,
    #[error("published competition category is empty or exceeds its bound")]
    InvalidCategory,
    #[error("published event specifications contradict one another")]
    ConflictingSpecification,
    #[error("published event label contains an invalid specification")]
    InvalidLabel,
}
