//! Row decision vocabulary: the supplied-evidence assessment, the decision that
//! evidence supports, and the review choice that can override an identity tie.
//!
//! The vocabulary is declared here; behavior lives in the child modules --
//! `pipeline` (grouping and decision), `review` (review transitions) and
//! `validation` (input bounds) -- while `matching` holds source/profile
//! agreement rules.

use crate::domain::{
    candidate::CandidateCoverage,
    identity::{AthleteId, EvidenceDigest},
};
use serde::{Deserialize, Serialize};

mod matching;
mod pipeline;
mod review;
mod validation;

pub use self::pipeline::assess;
pub use self::review::apply_review;

const MAX_SOURCE_TEXT_BYTES: usize = 4_096;
const MAX_PROFILES: usize = 4_096;
const MAX_SEARCH_REASONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchCompleteness {
    Complete { evidence: EvidenceDigest },
    Incomplete { reasons: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    DeterministicAccepted,
    IdentityReview,
    EvidenceReview,
    CompleteSearchNoMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateReason {
    athlete_id: AthleteId,
    exact_name: bool,
    matching_school: bool,
    location_corroborated: bool,
    hard_eligible: bool,
    participation_confirmed: bool,
    evidence_conflict: bool,
    evidence_strength: u8,
    reasons: Vec<String>,
    evidence: Vec<EvidenceDigest>,
    mailing_location_matches: bool,
    coverage: CandidateCoverage,
}

impl CandidateReason {
    #[must_use]
    pub fn athlete_id(&self) -> AthleteId {
        self.athlete_id
    }
    #[must_use]
    pub fn exact_name(&self) -> bool {
        self.exact_name
    }
    #[must_use]
    pub fn matching_school(&self) -> bool {
        self.matching_school
    }
    #[must_use]
    pub fn location_corroborated(&self) -> bool {
        self.location_corroborated
    }
    #[must_use]
    pub fn hard_eligible(&self) -> bool {
        self.hard_eligible
    }
    #[must_use]
    pub fn evidence_strength(&self) -> u8 {
        self.evidence_strength
    }
    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceDigest] {
        &self.evidence
    }
    #[must_use]
    pub fn participation_confirmed(&self) -> bool {
        self.participation_confirmed
    }
    #[must_use]
    pub fn evidence_conflict(&self) -> bool {
        self.evidence_conflict
    }
    #[must_use]
    pub fn coverage(&self) -> CandidateCoverage {
        self.coverage
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct VerifiedMatch {
    athlete_id: AthleteId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assessment {
    decision: Decision,
    candidates: Vec<CandidateReason>,
    search: SearchCompleteness,
    verified: Option<VerifiedMatch>,
}

impl Assessment {
    #[must_use]
    pub fn decision(&self) -> Decision {
        self.decision
    }
    #[must_use]
    pub fn candidates(&self) -> &[CandidateReason] {
        &self.candidates
    }
    #[must_use]
    pub fn search(&self) -> &SearchCompleteness {
        &self.search
    }
    #[must_use]
    pub fn accepted_athlete_id(&self) -> Option<AthleteId> {
        self.verified.as_ref().map(|matched| matched.athlete_id)
    }
    #[must_use]
    pub fn is_deterministic_acceptance(&self) -> bool {
        matches!(self.decision, Decision::DeterministicAccepted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewChoice {
    Select(AthleteId),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FinalDecision {
    accepted: Option<VerifiedMatch>,
}

impl FinalDecision {
    fn accepted(athlete_id: AthleteId) -> Self {
        Self {
            accepted: Some(VerifiedMatch { athlete_id }),
        }
    }

    #[must_use]
    pub fn accepted_athlete_id(&self) -> Option<AthleteId> {
        self.accepted.as_ref().map(|matched| matched.athlete_id)
    }
}

#[cfg(test)]
mod tests;
