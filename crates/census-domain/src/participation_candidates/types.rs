use super::ParticipationReasons;
use crate::affiliation_candidates::{
    AffiliationCandidateError, DeclaredFrozenView, KnownBounds, ObjectRef,
};
use crate::model::{
    AthleteId, CanonicalJsonError, ContractRefV1, GradYear, PerformanceId, ProfileDate,
    ProfileDateRangeV1, ProfileId, QueryProfileV1, SchoolYear, SourceIdentity,
};
use chrono::{DateTime, Utc};

pub const MAX_PARTICIPATION_FACTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateBinding<'a> {
    pub profile: &'a ProfileId,
    pub participation_rule: &'a ContractRefV1,
    pub frozen: DeclaredFrozenView<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct CandidateQuery<'a> {
    pub profile: &'a QueryProfileV1,
    pub binding: CandidateBinding<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofReference<'a> {
    pub fact: ObjectRef<'a>,
    pub capture: ObjectRef<'a>,
    pub interpretation: ObjectRef<'a>,
    pub locator: ObjectRef<'a>,
    pub athlete: &'a AthleteId,
    pub known: KnownBounds<'a>,
    pub acquired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultOwnership<'a> {
    Bound {
        performance: &'a PerformanceId,
        source_athlete: &'a SourceIdentity,
        source_result: ObjectRef<'a>,
        proof: ProofReference<'a>,
    },
    Missing,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceDateFact<'a> {
    Exact(&'a ProfileDate),
    Missing,
    YearOnly,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CohortFact<'a> {
    Resolved {
        year: GradYear,
        proof: ProofReference<'a>,
    },
    Missing,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipationContext {
    Scholastic,
    Club,
    Open,
    Unattached,
    Collegiate,
    MiddleSchool,
    PostGraduation,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextFact<'a> {
    pub context: ParticipationContext,
    pub proof: ProofReference<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentStatus {
    HighSchool,
    Collegiate,
    MiddleSchool,
    PostGraduation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBasis {
    ExplicitScholasticResult,
    DatedSchoolRoster,
    ExplicitStudentStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighSchoolStatusFact<'a> {
    pub status: StudentStatus,
    pub basis: StatusBasis,
    pub interval: &'a ProfileDateRangeV1,
    pub proof: ProofReference<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EligibilityWindowIdentity<'a> {
    pub school_year: SchoolYear,
    pub cohort: GradYear,
    pub participation_rule: &'a ContractRefV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EligibilityWindowFact<'a> {
    pub identity: EligibilityWindowIdentity<'a>,
    pub interval: &'a ProfileDateRangeV1,
    pub documented_summer_end: &'a ProfileDate,
    pub boundary_proof: ProofReference<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct ParticipationObservation<'a> {
    pub binding: CandidateBinding<'a>,
    pub performance: &'a PerformanceId,
    pub athlete: &'a AthleteId,
    pub ownership: ResultOwnership<'a>,
    pub date: PerformanceDateFact<'a>,
    pub cohort: CohortFact<'a>,
    pub contexts: &'a [ContextFact<'a>],
    pub high_school_status: &'a [HighSchoolStatusFact<'a>],
    pub eligibility_windows: &'a [EligibilityWindowFact<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutcome {
    EligibleCandidate,
    Excluded,
    Unresolved,
}

#[derive(Debug)]
pub struct CandidateAssessment<'a> {
    pub query: CandidateQuery<'a>,
    pub observation: &'a ParticipationObservation<'a>,
    pub outcome: CandidateOutcome,
    pub reasons: ParticipationReasons,
}

#[derive(Debug, thiserror::Error)]
pub enum ParticipationInputError {
    #[error("participation candidate profile identity does not match the selected profile")]
    ProfileMismatch,
    #[error("participation candidate rule does not match the selected profile")]
    PolicyMismatch,
    #[error("participation candidate frozen-generation binding differs")]
    FrozenBindingMismatch,
    #[error("participation candidate evidence names a foreign athlete or result")]
    ForeignSubject,
    #[error("participation candidate source athlete is missing or exceeds its envelope")]
    InvalidSourceAthlete,
    #[error("participation candidate facts exceed the {MAX_PARTICIPATION_FACTS}-fact envelope")]
    TooManyFacts,
    #[error("participation candidate reuses a proof identity with different metadata")]
    ConflictingProofIdentity,
    #[error("visible participation evidence was acquired after the declared frozen instant")]
    AcquisitionAfterFrozenInstant,
    #[error("participation interval does not end at its documented summer endpoint")]
    SummerEndpointMismatch,
    #[error("participation candidate temporal metadata is invalid: {0}")]
    Temporal(#[from] AffiliationCandidateError),
    #[error("participation profile identity could not be calculated: {0}")]
    Canonical(#[from] CanonicalJsonError),
}
