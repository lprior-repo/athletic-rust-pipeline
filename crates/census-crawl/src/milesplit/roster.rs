use serde::{Deserialize, Serialize};

use super::wire::Roster;
use crate::net::FetchOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRowLocator {
    pub ordinal: u32,
    pub byte_offset: usize,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterQuarantine {
    NotFound,
    UnknownTemplate,
    NoReadableRows,
    InvalidEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterRejectionKind {
    MissingIdentity,
    MissingName,
    MissingGraduationYear,
    InvalidGraduationYear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterRejection {
    pub row: SourceRowLocator,
    pub athlete_id: Option<String>,
    pub kind: RosterRejectionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterVerdict {
    Complete {
        roster: Roster,
    },
    Partial {
        roster: Roster,
        rejected: Vec<RosterRejection>,
    },
    Quarantined {
        reason: RosterQuarantine,
        rejected: Vec<RosterRejection>,
    },
}

impl RosterVerdict {
    pub fn roster(&self) -> Option<&Roster> {
        match self {
            Self::Complete { roster } | Self::Partial { roster, .. } => Some(roster),
            Self::Quarantined { .. } => None,
        }
    }

    pub fn rejections(&self) -> &[RosterRejection] {
        match self {
            Self::Complete { .. } => &[],
            Self::Partial { rejected, .. } | Self::Quarantined { rejected, .. } => rejected,
        }
    }
}

#[derive(Debug)]
pub struct RosterOutcome {
    pub capture: FetchOutcome,
    pub verdict: RosterVerdict,
}
