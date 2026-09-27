use serde::{Deserialize, Serialize};

use super::SchoolYear;
use super::SourceRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachTenure {
    Current {
        school_year: SchoolYear,
    },
    Former {
        last_school_year: Option<SchoolYear>,
    },
    Unknown,
}

impl CoachTenure {
    pub const fn is_current(&self) -> bool {
        matches!(self, Self::Current { .. })
    }

    pub const fn is_former(&self) -> bool {
        matches!(self, Self::Former { .. })
    }

    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachTenureEvidence {
    pub tenure: CoachTenure,
    pub source: SourceRef,
    pub source_sha256: String,
    pub retrieved_at: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TenureValidation {
    #[error("{field} is missing")]
    Missing { field: &'static str },
    #[error("{field} is empty")]
    Empty { field: &'static str },
    #[error("{field} is malformed")]
    Malformed { field: &'static str },
    #[error("{field} exceeds {limit} bytes")]
    TooLong { field: &'static str, limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TenureAssessmentError {
    #[error("invalid tenure evidence at index {index}: {source}")]
    InvalidEvidence {
        index: usize,
        #[source]
        source: TenureValidation,
    },
    #[error("current and former tenure claims conflict")]
    Conflict,
}

pub fn assess_coach_tenure<'a>(
    evidence: impl IntoIterator<Item = &'a CoachTenureEvidence>,
    school_year: SchoolYear,
) -> Result<CoachTenure, TenureAssessmentError> {
    let mut state = CoachTenure::Unknown;
    for (index, fact) in evidence.into_iter().enumerate() {
        validate_tenure_evidence(fact)
            .map_err(|source| TenureAssessmentError::InvalidEvidence { index, source })?;
        match fact.tenure {
            CoachTenure::Current { school_year: year } if year != school_year => continue,
            CoachTenure::Former {
                last_school_year: Some(year),
            } if year > school_year => continue,
            _ => {}
        }
        state = match (state, fact.tenure) {
            (CoachTenure::Unknown, claim) | (claim, CoachTenure::Unknown) => claim,
            (CoachTenure::Current { .. }, CoachTenure::Former { .. })
            | (CoachTenure::Former { .. }, CoachTenure::Current { .. }) => {
                return Err(TenureAssessmentError::Conflict);
            }
            (
                CoachTenure::Former {
                    last_school_year: left,
                },
                CoachTenure::Former {
                    last_school_year: right,
                },
            ) => CoachTenure::Former {
                last_school_year: left.max(right),
            },
            (current, _) => current,
        };
    }
    Ok(state)
}

pub fn validate_tenure_evidence(
    e: &CoachTenureEvidence,
) -> std::result::Result<(), TenureValidation> {
    if e.source.id.trim().is_empty() {
        return Err(TenureValidation::Missing { field: "source.id" });
    }
    if e.source_sha256.trim().is_empty() {
        return Err(TenureValidation::Empty {
            field: "source_sha256",
        });
    }
    if e.source_sha256.len() != 64 || !e.source_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(TenureValidation::Malformed {
            field: "source_sha256",
        });
    }
    if e.retrieved_at.trim().is_empty() {
        return Err(TenureValidation::Empty {
            field: "retrieved_at",
        });
    }
    if chrono::DateTime::parse_from_rfc3339(&e.retrieved_at).is_err() {
        return Err(TenureValidation::Malformed {
            field: "retrieved_at",
        });
    }
    if e.statement.trim().is_empty() {
        return Err(TenureValidation::Empty { field: "statement" });
    }
    if e.statement.len() > 512 {
        return Err(TenureValidation::TooLong {
            field: "statement",
            limit: 512,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
