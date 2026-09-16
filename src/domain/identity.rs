use crate::domain::error::DomainError;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use url::Url;
mod validation;
use validation::{
    canonical_profile_url, profile_path, validate_digest, validate_profile_origin, validate_sheet,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct WorkbookDigest(String);

impl WorkbookDigest {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for WorkbookDigest {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_digest(raw, "workbook_digest").map(Self)
    }
}

impl TryFrom<String> for WorkbookDigest {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_digest(&raw, "workbook_digest").map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for WorkbookDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_from(raw).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct EvidenceDigest(String);

impl EvidenceDigest {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for EvidenceDigest {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        validate_digest(raw, "evidence_digest").map(Self)
    }
}

impl TryFrom<String> for EvidenceDigest {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        validate_digest(&raw, "evidence_digest").map(|_| Self(raw))
    }
}

impl<'de> Deserialize<'de> for EvidenceDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_from(raw).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct AthleteId(u64);

impl AthleteId {
    pub fn new(value: u64) -> Result<Self, DomainError> {
        Self::try_from(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for AthleteId {
    type Error = DomainError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        (value > 0)
            .then_some(Self(value))
            .ok_or(DomainError::OutOfRange {
                field: "athlete_id",
            })
    }
}

impl<'de> Deserialize<'de> for AthleteId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::try_from(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceRowKey {
    raw: String,
    sheet_end: usize,
    row: u32,
}

impl SourceRowKey {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn sheet(&self) -> &str {
        self.raw.get(..self.sheet_end).map_or("", |sheet| sheet)
    }

    pub fn row(&self) -> u32 {
        self.row
    }
}

impl TryFrom<&str> for SourceRowKey {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        Self::try_from(raw.to_owned())
    }
}

impl TryFrom<String> for SourceRowKey {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let Some((sheet, row_text)) = raw.rsplit_once(':') else {
            return Err(DomainError::InvalidFormat {
                field: "source_row_key",
            });
        };
        validate_sheet(sheet)?;
        let row = row_text
            .parse::<u32>()
            .map_err(|_| DomainError::InvalidFormat {
                field: "source_row_key",
            })?;
        if row < 2 {
            return Err(DomainError::OutOfRange {
                field: "source_row_key_row",
            });
        }
        let sheet_end = sheet.len();
        Ok(Self {
            raw,
            sheet_end,
            row,
        })
    }
}

impl Serialize for SourceRowKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceRowKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_from(raw).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileUrl {
    canonical: String,
    athlete_id: AthleteId,
}

impl ProfileUrl {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        Self::try_from(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.canonical
    }

    pub fn athlete_id(&self) -> AthleteId {
        self.athlete_id
    }
}

impl TryFrom<&str> for ProfileUrl {
    type Error = DomainError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        let parsed = Url::parse(raw).map_err(|_| DomainError::InvalidFormat {
            field: "profile_url",
        })?;
        validate_profile_origin(raw, &parsed)?;
        let (id_text, suffix) = profile_path(parsed.path())?;
        let athlete_id_value = id_text
            .parse::<u64>()
            .map_err(|_| DomainError::InvalidFormat {
                field: "profile_url_athlete_id",
            })?;
        let athlete_id = AthleteId::new(athlete_id_value)?;
        let canonical = canonical_profile_url(athlete_id, suffix);
        Ok(Self {
            canonical,
            athlete_id,
        })
    }
}

impl TryFrom<String> for ProfileUrl {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::try_from(raw.as_str())
    }
}

impl<'de> Deserialize<'de> for ProfileUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_from(raw).map_err(D::Error::custom)
    }
}
impl Serialize for ProfileUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod identity_contract_tests;
