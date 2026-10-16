mod assess;
mod checks;
mod in_season;
mod policy;
mod reasons;
mod types;
mod windows;

pub use assess::{
    assess_historical_participation, assess_participation_for_policy,
    assess_participation_with_in_season,
};
pub use in_season::{
    participation_policy_v4, PARTICIPATION_POLICY_V4_DOCUMENT, PARTICIPATION_POLICY_V4_SHA256,
};
pub use policy::{
    participation_policy_v2, participation_policy_v3, PARTICIPATION_POLICY_V2_DOCUMENT,
    PARTICIPATION_POLICY_V2_SHA256, PARTICIPATION_POLICY_V3_DOCUMENT,
    PARTICIPATION_POLICY_V3_SHA256,
};
pub use reasons::{ParticipationReason, ParticipationReasons};
pub use types::{
    CandidateAssessment, CandidateBinding, CandidateOutcome, CandidateQuery, CohortFact,
    ContextFact, EligibilityWindowFact, EligibilityWindowIdentity, HighSchoolStatusFact,
    InSeasonEvidence, InSeasonWindowFact, ParticipationContext, ParticipationInputError,
    ParticipationObservation, PerformanceDateFact, ProofReference, ResultOwnership, StatusBasis,
    StudentStatus, MAX_PARTICIPATION_FACTS,
};

#[cfg(test)]
mod tests;
