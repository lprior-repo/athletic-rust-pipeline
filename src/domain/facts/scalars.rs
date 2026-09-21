//! Bounded numeric facts: graduation year, evidence-strength confidence score,
//! and retry count.

use crate::domain::error::DomainError;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

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
