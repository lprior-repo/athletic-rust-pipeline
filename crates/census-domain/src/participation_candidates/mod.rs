mod assess;
mod checks;
mod reasons;
mod types;
mod windows;

pub use assess::assess_participation;
pub use reasons::{ParticipationReason, ParticipationReasons};
pub use types::{
    CandidateAssessment, CandidateBinding, CandidateOutcome, CandidateQuery, CohortFact,
    ContextFact, EligibilityWindowFact, EligibilityWindowIdentity, HighSchoolStatusFact,
    ParticipationContext, ParticipationInputError, ParticipationObservation, PerformanceDateFact,
    ProofReference, ResultOwnership, StatusBasis, StudentStatus, MAX_PARTICIPATION_FACTS,
};

#[cfg(test)]
mod tests;
