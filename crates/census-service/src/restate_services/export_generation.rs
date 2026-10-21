use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

pub const DEFAULT_GENERATION: &str = "1";
pub const MAX_EXPORT_GENERATION_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportGeneration(String);

#[derive(Debug, thiserror::Error)]
pub enum ExportGenerationError {
    #[error("export generation must not be empty")]
    Empty,
    #[error("export generation is {value} bytes, longer than {MAX_EXPORT_GENERATION_LEN}")]
    TooLong { value: usize },
    #[error("export generation {value:?} carries a character outside [A-Za-z0-9._-]")]
    InvalidChar { value: String },
    #[error("export key part {value:?} must not contain ':'")]
    InvalidKeyPart { value: String },
    #[error("selected export request could not be serialized for workflow identity: {0}")]
    SelectedRequestDigest(#[source] census_domain::model::CanonicalJsonError),
    #[error("--generation {value:?} is live-Restate only")]
    OfflineGeneration { value: String },
    #[error("export generation requires an explicit --generation value; omitted generations no longer default to legacy 1")]
    Missing,
}

impl ExportGeneration {
    pub fn parse(value: &str) -> Result<Self, ExportGenerationError> {
        Self::checked(value)
    }

    fn checked(value: impl AsRef<str> + Into<String>) -> Result<Self, ExportGenerationError> {
        let text = value.as_ref();
        if text.is_empty() {
            return Err(ExportGenerationError::Empty);
        }
        if text.len() > MAX_EXPORT_GENERATION_LEN {
            return Err(ExportGenerationError::TooLong { value: text.len() });
        }
        if !text.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-' || byte == b'_'
        }) {
            return Err(ExportGenerationError::InvalidChar {
                value: value.into(),
            });
        }
        Ok(Self(value.into()))
    }

    pub fn default_generation() -> Self {
        Self(DEFAULT_GENERATION.to_owned())
    }

    pub fn resolve(flag: Option<&str>) -> Result<Self, ExportGenerationError> {
        match flag {
            Some(value) => Self::parse(value),
            None => Err(ExportGenerationError::Missing),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ExportGeneration {
    type Error = ExportGenerationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::checked(value)
    }
}

struct GenerationVisitor;

impl<'de> Visitor<'de> for GenerationVisitor {
    type Value = ExportGeneration;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded ASCII export generation")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        ExportGeneration::parse(value).map_err(E::custom)
    }

    fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
        ExportGeneration::try_from(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for ExportGeneration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(GenerationVisitor)
    }
}

pub fn reject_offline_generation(flag: Option<&str>) -> Result<(), ExportGenerationError> {
    match flag {
        Some(value) => Err(ExportGenerationError::OfflineGeneration {
            value: value.to_owned(),
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn omitted_generation_refuses_instead_of_defaulting_to_legacy() {
        assert!(matches!(
            ExportGeneration::resolve(None),
            Err(ExportGenerationError::Missing)
        ));
    }

    #[test]
    fn explicit_generation_parses() -> Result<(), Box<dyn std::error::Error>> {
        let generation = ExportGeneration::resolve(Some("qualified-v1"))?;
        check!(eq; generation.as_str(), "qualified-v1");
        Ok(())
    }
}
