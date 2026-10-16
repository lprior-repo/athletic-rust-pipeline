use super::checks::visible;
use super::policy::ParticipationPolicy;
use super::{
    CandidateQuery, CohortFact, EligibilityWindowFact, InSeasonEvidence, ParticipationInputError,
    ParticipationObservation, ParticipationReason, ParticipationReasons,
};
use crate::model::ProfileDate;

pub(super) fn assess(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    date: &ProfileDate,
    reasons: &mut ParticipationReasons,
    policy: ParticipationPolicy,
    season: InSeasonEvidence<'_>,
) -> Result<(), ParticipationInputError> {
    let in_season = super::in_season::admitted(query, observation, date, season, reasons)?;
    let (seen, admitted, conflicted) = observation.eligibility_windows.iter().try_fold(
        (false, false, false),
        |(seen, admitted, conflicted), fact| {
            if !applicable(query, observation, fact)? {
                return Ok((seen, admitted, conflicted));
            }
            let competing = competing_boundary(query, observation, fact, date)?;
            Ok::<_, ParticipationInputError>((
                true,
                admitted || fact.interval.contains(date),
                conflicted || competing,
            ))
        },
    )?;
    if conflicted {
        reasons.add(ParticipationReason::ParticipationWindowConflict);
    }
    if !seen {
        if !(matches!(policy, ParticipationPolicy::SourceProvenInSeason) && in_season)
            && !policy.establishes_date_local_status(query, observation, date, *reasons)?
        {
            reasons.add(ParticipationReason::SummerBoundaryUnresolved);
        }
    } else if !admitted {
        reasons.add(ParticipationReason::OutsideParticipationWindow);
    }
    Ok(())
}

fn applicable(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    fact: &EligibilityWindowFact<'_>,
) -> Result<bool, ParticipationInputError> {
    if !visible(query, fact.boundary_proof)?
        || fact.identity.participation_rule != query.binding.participation_rule
        || !query
            .profile
            .input()
            .graduation_years
            .contains(&fact.identity.cohort)
    {
        return Ok(false);
    }
    if let CohortFact::Resolved { year, .. } = observation.cohort {
        if fact.identity.cohort != year {
            return Ok(false);
        }
    }
    Ok(true)
}

fn competing_boundary(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    fact: &EligibilityWindowFact<'_>,
    date: &ProfileDate,
) -> Result<bool, ParticipationInputError> {
    observation
        .eligibility_windows
        .iter()
        .try_fold(false, |found, other| {
            Ok(found
                || (fact.identity == other.identity
                    && fact.interval.contains(date) != other.interval.contains(date)
                    && applicable(query, observation, other)?))
        })
}
