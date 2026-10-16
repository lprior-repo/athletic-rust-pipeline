use super::invalid;
use census_domain::model::CanonicalPerformance;
use census_domain::participation_candidates::{
    assess_participation_for_policy, assess_participation_with_in_season, CandidateOutcome,
    CandidateQuery, CohortFact, InSeasonEvidence, ParticipationObservation, ParticipationReasons,
    PerformanceDateFact, ResultOwnership, PARTICIPATION_POLICY_V4_SHA256,
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
    let is_v4 = query.binding.participation_rule.revision().get() == 4
        && query.binding.participation_rule.digest() == PARTICIPATION_POLICY_V4_SHA256;
    let assessment = if is_v4 {
        assess_participation_with_in_season(query, &observation, InSeasonEvidence::Unavailable)
    } else {
        assess_participation_for_policy(query, &observation)
    }
    .map_err(|_| invalid("strict participation assessor rejected its pinned profile binding"))?;
    Ok((assessment.outcome, assessment.reasons))
}
