mod assess;
mod checks;
mod policy;
mod reasons;
mod types;
mod windows;

pub use assess::assess_historical_participation as assess_participation;
pub use assess::{assess_historical_participation, assess_participation_for_policy};
pub use policy::{
    participation_policy_v2, PARTICIPATION_POLICY_V2_DOCUMENT, PARTICIPATION_POLICY_V2_SHA256,
};
pub use reasons::{ParticipationReason, ParticipationReasons};
pub use types::{
    CandidateAssessment, CandidateBinding, CandidateOutcome, CandidateQuery, CohortFact,
    ContextFact, EligibilityWindowFact, EligibilityWindowIdentity, HighSchoolStatusFact,
    ParticipationContext, ParticipationInputError, ParticipationObservation, PerformanceDateFact,
    ProofReference, ResultOwnership, StatusBasis, StudentStatus, MAX_PARTICIPATION_FACTS,
};

#[cfg(test)]
mod tests;
