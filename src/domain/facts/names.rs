//! Display-name facts: source text preserved verbatim behind length and
//! control-character validation.

use crate::domain::error::DomainError;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use super::text_validation::validate_text;

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
