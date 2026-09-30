use serde::{Deserialize, Serialize};

use super::text::{collapse_whitespace, reject_control};
use super::DirectoryError;

pub const PHONE_LIMIT: usize = 32;

pub const WEBSITE_LIMIT: usize = 200;

const PHONE_DIGITS_MIN: usize = 7;

const PHONE_DIGITS_MAX: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Phone(String);

impl Phone {
    pub fn parse(raw: &str) -> Result<Option<Self>, DirectoryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        reject_control(trimmed, "phone")?;
        let value = collapse_whitespace(trimmed);
        let length = value.chars().count();
        if length > PHONE_LIMIT {
            return Err(DirectoryError::FieldTooLong {
                field: "phone",
                limit: PHONE_LIMIT,
                length,
            });
        }
        let digits = value.chars().filter(char::is_ascii_digit).count();
        if !(PHONE_DIGITS_MIN..=PHONE_DIGITS_MAX).contains(&digits) {
            return Err(DirectoryError::UnsupportedValue {
                field: "phone",
                value: raw.to_string(),
            });
        }
        Ok(Some(Self(value)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Phone {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?.ok_or(DirectoryError::EmptyField { field: "phone" })?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical phone",
                value: raw,
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Website(String);

impl Website {
    pub fn parse(raw: &str) -> Result<Option<Self>, DirectoryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        reject_control(trimmed, "website")?;
        let length = trimmed.chars().count();
        if length > WEBSITE_LIMIT {
            return Err(DirectoryError::FieldTooLong {
                field: "website",
                limit: WEBSITE_LIMIT,
                length,
            });
        }
        let scheme_ok = trimmed.starts_with("http://") || trimmed.starts_with("https://");
        let body_ok = trimmed.chars().all(|ch| !ch.is_whitespace());
        if !scheme_ok || !body_ok {
            return Err(DirectoryError::UnsupportedValue {
                field: "website",
                value: raw.to_string(),
            });
        }
        Ok(Some(Self(trimmed.to_string())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Website {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?.ok_or(DirectoryError::EmptyField { field: "website" })?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical website",
                value: raw,
            });
        }
        Ok(value)
    }
}
