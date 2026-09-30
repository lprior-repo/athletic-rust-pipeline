use serde::{Deserialize, Serialize};

use super::name::AssociationLabel;
use super::DirectoryError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SchoolKind {
    Public {
        charter: bool,
    },
    Private {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        affiliation: Option<AssociationLabel>,
    },
}

impl SchoolKind {
    pub fn public() -> Self {
        Self::Public { charter: false }
    }

    pub fn charter() -> Self {
        Self::Public { charter: true }
    }

    pub fn private() -> Self {
        Self::Private { affiliation: None }
    }

    pub fn affiliated(affiliation: AssociationLabel) -> Self {
        Self::Private {
            affiliation: Some(affiliation),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Public { charter: false } => "Public".to_string(),
            Self::Public { charter: true } => "Charter".to_string(),
            Self::Private { affiliation: None } => "Private".to_string(),
            Self::Private {
                affiliation: Some(affiliation),
            } => format!("Private ({})", affiliation.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8")]
pub struct NumberedGrade(u8);

impl NumberedGrade {
    pub fn new(number: u8) -> Result<Self, DirectoryError> {
        if (1..=12).contains(&number) {
            Ok(Self(number))
        } else {
            Err(DirectoryError::UnsupportedGrade {
                value: number.to_string(),
            })
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for NumberedGrade {
    type Error = DirectoryError;

    fn try_from(number: u8) -> Result<Self, Self::Error> {
        Self::new(number)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Grade {
    PreK,
    Kindergarten,
    Numbered(NumberedGrade),
    Ungraded,
    AdultEducation,
}

impl Grade {
    pub fn parse(raw: &str) -> Result<Option<Self>, DirectoryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("n") {
            return Ok(None);
        }
        let upper = trimmed.to_ascii_uppercase();
        match upper.as_str() {
            "PK" | "PREK" | "PRE-K" | "PRE K" | "P" => Ok(Some(Self::PreK)),
            "KG" | "K" | "KINDERGARTEN" => Ok(Some(Self::Kindergarten)),
            "UG" | "UNGRADED" => Ok(Some(Self::Ungraded)),
            "AE" | "ADULT" | "ADULT EDUCATION" => Ok(Some(Self::AdultEducation)),
            other => match other.parse::<u8>() {
                Ok(number) => NumberedGrade::new(number).map(Self::Numbered).map(Some),
                _ => Err(DirectoryError::UnsupportedGrade {
                    value: raw.to_string(),
                }),
            },
        }
    }

    pub fn rank(self) -> Option<u8> {
        match self {
            Self::PreK => Some(0),
            Self::Kindergarten => Some(1),
            Self::Numbered(number) => Some(number.get().saturating_add(1)),
            Self::Ungraded | Self::AdultEducation => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::PreK => "PK".to_string(),
            Self::Kindergarten => "KG".to_string(),
            Self::Numbered(number) => number.get().to_string(),
            Self::Ungraded => "UG".to_string(),
            Self::AdultEducation => "AE".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct GradeSpan {
    low: Grade,
    high: Grade,
}

impl GradeSpan {
    pub fn new(low: Grade, high: Grade) -> Result<Self, DirectoryError> {
        match (low.rank(), high.rank()) {
            (Some(low_rank), Some(high_rank)) if low_rank <= high_rank => Ok(Self { low, high }),
            (Some(_), Some(_)) => Err(DirectoryError::GradeSpanInverted {
                low: low.label(),
                high: high.label(),
            }),
            _ => Err(DirectoryError::UnsupportedGrade {
                value: format!("{}..{}", low.label(), high.label()),
            }),
        }
    }

    pub fn low(self) -> Grade {
        self.low
    }

    pub fn high(self) -> Grade {
        self.high
    }

    pub fn label(self) -> String {
        format!("{}-{}", self.low.label(), self.high.label())
    }
}

impl Serialize for GradeSpan {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for GradeSpan {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let span = Self::deserialize(deserializer)?;
        Self::new(span.low, span.high).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Enrollment(u32);

impl Enrollment {
    pub fn parse(raw: &str) -> Result<Option<Self>, DirectoryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        match trimmed.parse::<u32>() {
            Ok(value) => Ok(Some(Self(value))),
            Err(_) => Err(DirectoryError::MalformedInteger {
                field: "enrollment",
                value: raw.to_string(),
            }),
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}
