use crate::domain::error::DomainError;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
mod text_validation;
use text_validation::validate_text;

const MAX_ATHLETE_NAME_BYTES: usize = 200;
const MAX_SCHOOL_NAME_BYTES: usize = 200;
const MAX_CITY_NAME_BYTES: usize = 120;
const MAX_REGION_NAME_BYTES: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AthleteName(String);

impl AthleteName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AthleteName {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_text(raw, "athlete_name", MAX_ATHLETE_NAME_BYTES).map(|_| Self(raw.to_owned()))
    }
}

impl TryFrom<String> for AthleteName {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_text(&raw, "athlete_name", MAX_ATHLETE_NAME_BYTES).map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for AthleteName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SchoolName(String);

impl SchoolName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for SchoolName {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_text(raw, "school_name", MAX_SCHOOL_NAME_BYTES).map(|_| Self(raw.to_owned()))
    }
}

impl TryFrom<String> for SchoolName {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_text(&raw, "school_name", MAX_SCHOOL_NAME_BYTES).map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for SchoolName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CityName(String);

impl CityName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CityName {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_text(raw, "city_name", MAX_CITY_NAME_BYTES).map(|_| Self(raw.to_owned()))
    }
}

impl TryFrom<String> for CityName {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_text(&raw, "city_name", MAX_CITY_NAME_BYTES).map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for CityName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RegionName(String);

impl RegionName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for RegionName {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_text(raw, "region_name", MAX_REGION_NAME_BYTES).map(|_| Self(raw.to_owned()))
    }
}

impl TryFrom<String> for RegionName {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_text(&raw, "region_name", MAX_REGION_NAME_BYTES).map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for RegionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct GraduationYear(u16);

impl GraduationYear {
    pub fn new(value: u16) -> Result<Self, DomainError> {
        Self::try_from(value)
    }

    pub fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for GraduationYear {
    type Error = DomainError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        (1900..=2200)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(DomainError::OutOfRange {
                field: "graduation_year",
            })
    }
}

impl<'de> Deserialize<'de> for GraduationYear {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(u16::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// A deterministic evidence-strength score, not a calibrated probability.
    pub fn new(value: f64) -> Result<Self, DomainError> {
        Self::try_from(value)
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for ConfidenceScore {
    type Error = DomainError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        value
            .is_finite()
            .then_some(value)
            .filter(|score| (0.0..=1.0).contains(score))
            .map(Self)
            .ok_or(DomainError::OutOfRange {
                field: "confidence_score",
            })
    }
}

impl<'de> Deserialize<'de> for ConfidenceScore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(f64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RetryCount(u8);

impl RetryCount {
    pub fn new(value: u8) -> Result<Self, DomainError> {
        Self::try_from(value)
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn next(self) -> Result<Self, DomainError> {
        match self.0 {
            0..=2 => Self::new(self.0.saturating_add(1)),
            3 => Err(DomainError::RetryExhausted),
            _ => Err(DomainError::OutOfRange {
                field: "retry_count",
            }),
        }
    }
}

impl TryFrom<u8> for RetryCount {
    type Error = DomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        (value <= 3)
            .then_some(Self(value))
            .ok_or(DomainError::OutOfRange {
                field: "retry_count",
            })
    }
}

impl<'de> Deserialize<'de> for RetryCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(u8::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Missing,
    CityOnly(CityName),
    RegionOnly(RegionName),
    CityRegion { city: CityName, region: RegionName },
}
impl Location {
    pub(crate) fn fields(&self) -> (Option<&CityName>, Option<&RegionName>) {
        match self {
            Self::Missing => (None, None),
            Self::CityOnly(city) => (Some(city), None),
            Self::RegionOnly(region) => (None, Some(region)),
            Self::CityRegion { city, region } => (Some(city), Some(region)),
        }
    }

    pub(crate) fn compatible_with(&self, other: &Self) -> bool {
        let (city, region) = self.fields();
        let (other_city, other_region) = other.fields();
        let city_compatible = match (city, other_city) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        };
        let region_compatible = match (region, other_region) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        };
        city_compatible && region_compatible
    }
}
#[cfg(test)]
mod facts_contract_tests;
