use serde::{Deserialize, Serialize};

use super::text::{as_is, bounded};
use super::DirectoryError;

pub const NAME_LIMIT: usize = 160;

pub const LABEL_LIMIT: usize = 48;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SchoolName(String);

impl SchoolName {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        Ok(Self(bounded(raw, "school name", NAME_LIMIT, as_is)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SchoolName {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical school name",
                value: raw,
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct AssociationLabel(String);

impl AssociationLabel {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        Ok(Self(bounded(raw, "association", LABEL_LIMIT, as_is)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AssociationLabel {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical association",
                value: raw,
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct MatchForm(String);

impl MatchForm {
    pub fn of(value: &str) -> Self {
        let mut out = String::with_capacity(value.len());
        let mut separator = false;
        for ch in value.chars() {
            if ch.is_alphanumeric() {
                if separator && !out.is_empty() {
                    out.push(' ');
                }
                separator = false;
                out.extend(ch.to_lowercase().filter(|lower| lower.is_alphanumeric()));
            } else if ch.is_whitespace() {
                separator = !out.is_empty();
            }
        }
        Self(out)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<String> for MatchForm {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::of(&raw);
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical matching form",
                value: raw,
            });
        }
        Ok(value)
    }
}
