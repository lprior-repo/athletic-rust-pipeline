use super::{
    facts::{AthleteName, GraduationYear, Location, SchoolName},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sport {
    TrackField,
    CrossCountry,
}

impl Sport {
    pub fn api_code(self) -> &'static str {
        match self {
            Self::TrackField => "tf",
            Self::CrossCountry => "xc",
        }
    }
}

/// A locator into retained source bytes, never a claim copied from the prospect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub document: EvidenceDigest,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observed<T> {
    pub value: T,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamEvidence {
    pub team_id: u64,
    pub name: Observed<SchoolName>,
    pub location: Option<Observed<Location>>,
    pub seasons: Vec<u16>,
    pub level: Option<u8>,
}

/// Grade observations are not graduation-year witnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradeAtSeason {
    pub team_id: u64,
    pub season: u16,
    pub grade: u8,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum SportAvailability {
    ResultsObserved { sport: Sport, count: u64 },
    EmptyResponse { sport: Sport },
    Unavailable { sport: Sport },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ResultAttribution {
    Individual,
    VerifiedRelayMember { relay_athlete_id: u64 },
    Unresolved { reported_athlete_id: Option<u64> },
}
/// Numeric TF best flags are opaque until the source's interpretation is verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
pub enum BestClaim {
    Claimed,
    NotClaimed,
    OpaqueFlags(u64),
    Unavailable,
}

/// Raw display marks and event metadata survive even when a mark is unsupported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultEvidence {
    pub result_id: u64,
    pub sport: Sport,
    pub event_id: Option<u64>,
    pub event_name: String,
    pub event_description: Option<String>,
    pub event_type: Option<String>,
    pub mark: String,
    pub units: Option<String>,
    pub season: u16,
    pub team_id: u64,
    pub meet_id: u64,
    pub meet_name: Option<String>,
    pub date: Option<String>,
    pub wind: Option<String>,
    pub timing: Option<String>,
    pub personal_best: BestClaim,
    pub season_best: BestClaim,
    pub attribution: ResultAttribution,
    pub short_code: Option<String>,
    pub result_url: Option<String>,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceIssue {
    pub code: String,
    pub message: String,
    pub evidence: Option<EvidenceRef>,
}

impl EvidenceIssue {
    /// Optional school-grade and graduation observations never determine workbook eligibility.
    #[must_use]
    pub fn is_descriptive_cohort(&self) -> bool {
        matches!(
            self.code.as_str(),
            "cohort_conflict"
                | "grade_conflict"
                | "unknown_grades_shape"
                | "grades_truncated"
                | "invalid_grade_key"
                | "invalid_grade"
                | "invalid_graduation_year"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub athlete_id: AthleteId,
    pub profile_url: ProfileUrl,
    pub name: Observed<AthleteName>,
    pub teams: Vec<TeamEvidence>,
    pub graduation_years: Vec<Observed<GraduationYear>>,
    pub grades: Vec<GradeAtSeason>,
    pub sports: Vec<SportAvailability>,
    pub results: Vec<ResultEvidence>,
    pub issues: Vec<EvidenceIssue>,
    pub documents: Vec<EvidenceDigest>,
}
