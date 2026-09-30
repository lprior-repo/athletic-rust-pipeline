use serde::{Deserialize, Serialize};

use super::text::reject_control;
use super::DirectoryError;

pub const NCES_ID_DIGITS: usize = 12;

pub const PSS_ID_LENGTH: usize = 8;

pub const STATE_ID_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct NcesSchoolId(String);

impl NcesSchoolId {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        digits_field(raw, NCES_ID_DIGITS, "NCES school id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NcesSchoolId {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical NCES school id",
                value: raw,
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct PssId(String);

impl PssId {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        let trimmed = raw.trim();
        let shaped = trimmed.chars().count() == PSS_ID_LENGTH
            && trimmed.chars().all(|ch| ch.is_ascii_alphanumeric());
        if !shaped {
            return Err(DirectoryError::UnsupportedValue {
                field: "PSS id",
                value: raw.to_string(),
            });
        }
        Ok(Self(trimmed.to_ascii_uppercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PssId {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical PSS id",
                value: raw,
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct StateRecordId(String);

impl StateRecordId {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DirectoryError::EmptyField {
                field: "state record id",
            });
        }
        reject_control(trimmed, "state record id")?;
        let length = trimmed.chars().count();
        if length > STATE_ID_LIMIT {
            return Err(DirectoryError::FieldTooLong {
                field: "state record id",
                limit: STATE_ID_LIMIT,
                length,
            });
        }
        if trimmed.chars().any(char::is_whitespace) {
            return Err(DirectoryError::UnsupportedValue {
                field: "state record id",
                value: raw.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for StateRecordId {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical state record id",
                value: raw,
            });
        }
        Ok(value)
    }
}

fn digits_field(raw: &str, length: usize, field: &'static str) -> Result<String, DirectoryError> {
    let trimmed = raw.trim();
    let digits_only = trimmed.chars().all(|ch| ch.is_ascii_digit());
    if trimmed.chars().count() != length || !digits_only {
        return Err(DirectoryError::UnsupportedValue {
            field,
            value: raw.to_string(),
        });
    }
    Ok(trimmed.to_string())
}
