use super::invalid;
use census_domain::model::CanonicalPerformance;
use census_domain::participation_candidates::{
    assess_participation_for_policy, CandidateOutcome, CandidateQuery, CohortFact,
    ParticipationObservation, ParticipationReasons, PerformanceDateFact, ResultOwnership,
};
use census_store::StoreResult;

pub(super) fn assess(
    query: CandidateQuery<'_>,
    performance: &CanonicalPerformance,
) -> StoreResult<(CandidateOutcome, ParticipationReasons)> {
    let observation = ParticipationObservation {
        binding: query.binding,
        performance: &performance.id,
        athlete: &performance.athlete,
        ownership: ResultOwnership::Missing,
        date: PerformanceDateFact::Missing,
        cohort: CohortFact::Missing,
        contexts: &[],
        high_school_status: &[],
        eligibility_windows: &[],
    };
    let assessment = assess_participation_for_policy(query, &observation).map_err(|_| {
        invalid("strict participation assessor rejected its pinned profile binding")
    })?;
    Ok((assessment.outcome, assessment.reasons))
}
