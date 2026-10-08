use super::{published_email, CoachContactProgram, SchoolId, SchoolYear, SourceRef};
use serde::{Deserialize, Serialize};
mod boundary;
mod selection;

pub use selection::{mailbox, mailbox_research, research, source_research};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchoolMailboxPurpose {
    SchoolOffice,
    AthleticsOffice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SchoolMailboxWire")]
pub struct SchoolMailboxClaim {
    pub school: SchoolId,
    pub purpose: SchoolMailboxPurpose,
    pub mailbox: String,
    pub school_year: SchoolYear,
    pub source: SourceRef,
    pub source_sha256: String,
    pub acquired_at: String,
    pub statement: String,
}

#[derive(Deserialize)]
struct SchoolMailboxWire {
    school: SchoolId,
    purpose: SchoolMailboxPurpose,
    mailbox: String,
    school_year: SchoolYear,
    source: SourceRef,
    source_sha256: String,
    acquired_at: String,
    statement: String,
}

impl SchoolMailboxClaim {
    pub fn validate(&self) -> Result<(), SchoolContactError> {
        self.validated_at().map(|_| ())
    }

    pub(super) fn validated_at(
        &self,
    ) -> Result<chrono::DateTime<chrono::FixedOffset>, SchoolContactError> {
        published_email(&self.mailbox).ok_or(SchoolContactError::InvalidMailbox)?;
        boundary::capture(
            &self.source,
            &self.source_sha256,
            &self.acquired_at,
            &self.statement,
        )
    }
}

impl TryFrom<SchoolMailboxWire> for SchoolMailboxClaim {
    type Error = SchoolContactError;
    fn try_from(row: SchoolMailboxWire) -> Result<Self, Self::Error> {
        let claim = Self {
            school: row.school,
            purpose: row.purpose,
            mailbox: row.mailbox,
            school_year: row.school_year,
            source: row.source,
            source_sha256: row.source_sha256,
            acquired_at: row.acquired_at,
            statement: row.statement,
        };
        claim.validate()?;
        Ok(claim)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactResearchOutcome {
    Unattempted,
    CompletedEmpty,
    CompletedClaims,
    Partial,
    Blocked,
    Failed,
    Exhausted,
    Stale,
    Ambiguous,
    Conflict,
}

impl ContactResearchOutcome {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::CompletedEmpty | Self::CompletedClaims)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unattempted => "unattempted",
            Self::CompletedEmpty => "completed_empty",
            Self::CompletedClaims => "completed_claims",
            Self::Partial => "partial",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Exhausted => "exhausted",
            Self::Stale => "stale",
            Self::Ambiguous => "ambiguous",
            Self::Conflict => "conflict",
        }
    }

    pub fn combine(self, other: Self) -> Self {
        if other.priority() > self.priority() {
            other
        } else {
            self
        }
    }

    fn priority(&self) -> u8 {
        match self {
            Self::CompletedEmpty => 0,
            Self::Exhausted => 1,
            Self::CompletedClaims => 2,
            Self::Unattempted => 3,
            Self::Stale => 4,
            Self::Partial => 5,
            Self::Failed => 6,
            Self::Blocked => 7,
            Self::Ambiguous => 8,
            Self::Conflict => 9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactResearchSubject {
    Program(CoachContactProgram),
    SchoolMailbox(SchoolMailboxPurpose),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactResearch {
    pub school: SchoolId,
    pub subject: ContactResearchSubject,
    pub school_year: SchoolYear,
    pub outcome: ContactResearchOutcome,
    pub attempts: Vec<ContactResearchAttempt>,
}

impl ContactResearch {
    pub fn programs() -> [CoachContactProgram; 7] {
        use super::{Gender, Sport};
        [
            CoachContactProgram::SchoolAthletics,
            CoachContactProgram::Team {
                sport: Sport::CrossCountry,
                gender: Gender::Boys,
            },
            CoachContactProgram::Team {
                sport: Sport::CrossCountry,
                gender: Gender::Girls,
            },
            CoachContactProgram::Team {
                sport: Sport::IndoorTrack,
                gender: Gender::Boys,
            },
            CoachContactProgram::Team {
                sport: Sport::IndoorTrack,
                gender: Gender::Girls,
            },
            CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: Gender::Boys,
            },
            CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: Gender::Girls,
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactResearchAttempt {
    pub locator: String,
    pub acquired_at: String,
    pub source_sha256: Option<String>,
    pub outcome: ContactResearchOutcome,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SchoolContactError {
    #[error("generic school mailbox must be an explicitly published professional address")]
    InvalidMailbox,
    #[error("school mailbox claim lacks a valid immutable capture and acquisition timestamp")]
    InvalidCapture,
    #[error("school mailbox claim belongs to another school")]
    ForeignSchool,
    #[error("current school mailbox claims conflict")]
    Conflict,
    #[error("school mailbox research has not completed: {0}")]
    ResearchIncomplete(&'static str),
}
