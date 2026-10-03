use crate::net::FetchOutcome;
use census_domain::model::{EventKind, Gender, GradYear, Mark, SourceIdentity, TimingMethod};
use serde::{Deserialize, Serialize};

mod context;
mod effect;
mod parse;
mod row;
#[cfg(test)]
mod tests;

pub(crate) use effect::{acquisition_manifest_key, read_owned_meet};
pub use parse::parse_owned_meet;

pub const OWNED_MEET_PHASE: &str = "milesplit_owned_meet_v3";
pub const OWNED_CAPTURE_PHASE: &str = "milesplit_owned_capture_v1";
pub const OWNED_FIELDS: &str = "id,meetId,teamId,teamName,athleteId,firstName,lastName,gender,divisionId,divisionName,meetResultsId,meetResultsDivisionId,resultsDivisionId,gradYear,eventName,eventCode,eventDistance,round,roundName,heat,units,mark,place,windReading,profileUrl,statusCode";
pub const MAX_OWNED_BODY_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_OWNED_ROWS: usize = 20_000;

#[derive(Debug)]
pub struct OwnedMeetOutcome {
    pub capture: FetchOutcome,
    pub verdict: OwnedMeetVerdict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum OwnedMeetVerdict {
    Parsed(OwnedMeetPage),
    Malformed { detail: String },
    Refused { status: u16 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnedMeetPage {
    pub rows: Vec<OwnedPerformance>,
    pub rejected: Vec<OwnedRejection>,
    pub published_rows: usize,
    pub completeness: OwnedCompleteness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedCompleteness {
    Unknown,
    ExplicitTotal { total: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnedPerformance {
    pub locator: String,
    pub result_id: u64,
    pub meet_id: u64,
    pub result_set_id: u64,
    pub source_athlete: SourceIdentity,
    pub team_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub gender: Gender,
    pub grad_year: Option<GradYear>,
    pub cohort: OwnedCohort,
    pub event_kind: EventKind,
    pub mark: Mark,
    pub timing: Option<TimingMethod>,
    pub provider: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedCohort {
    Published,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedRejection {
    pub locator: String,
    pub kind: OwnedRejectionKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedRejectionKind {
    MalformedRow,
    MissingOwner,
    InvalidIdentity,
    ForeignMeet,
    ProfileMismatch,
    InvalidContext,
    DuplicateResult,
    TeamRelay,
}

impl OwnedMeetPage {
    pub fn individual_parse_complete(&self) -> bool {
        self.rejected
            .iter()
            .all(|row| row.kind == OwnedRejectionKind::TeamRelay)
            && self.rows.len().checked_add(self.rejected.len()) == Some(self.published_rows)
    }

    pub fn ownership_complete(&self) -> bool {
        self.individual_parse_complete()
            && matches!(self.completeness, OwnedCompleteness::ExplicitTotal { total } if total == self.published_rows)
    }
}
