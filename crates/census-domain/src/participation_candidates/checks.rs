use super::{
    CandidateQuery, CohortFact, InSeasonEvidence, ParticipationInputError,
    ParticipationObservation, ProofReference, ResultOwnership, MAX_PARTICIPATION_FACTS,
};

pub(super) fn check(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
) -> Result<(), ParticipationInputError> {
    check_with_in_season(query, observation, InSeasonEvidence::Unavailable)
}

pub(super) fn check_with_in_season(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    season: InSeasonEvidence<'_>,
) -> Result<(), ParticipationInputError> {
    if query.profile.id()? != *query.binding.profile
        || observation.binding.profile != query.binding.profile
    {
        return Err(ParticipationInputError::ProfileMismatch);
    }
    if &query.profile.input().policies.participation != query.binding.participation_rule
        || observation.binding.participation_rule != query.binding.participation_rule
    {
        return Err(ParticipationInputError::PolicyMismatch);
    }
    if observation.binding.frozen != query.binding.frozen {
        return Err(ParticipationInputError::FrozenBindingMismatch);
    }
    let count = observation
        .contexts
        .len()
        .checked_add(observation.high_school_status.len())
        .and_then(|count| count.checked_add(observation.eligibility_windows.len()))
        .and_then(|count| count.checked_add(super::in_season::fact_count(season)))
        .and_then(|count| count.checked_add(2))
        .ok_or(ParticipationInputError::TooManyFacts)?;
    if count > MAX_PARTICIPATION_FACTS {
        return Err(ParticipationInputError::TooManyFacts);
    }
    super::in_season::check_facts(season)?;
    check_ownership(observation)?;
    check_proofs(query, observation, season)?;
    observation
        .eligibility_windows
        .iter()
        .try_for_each(|window| {
            if window.interval.through() != window.documented_summer_end {
                Err(ParticipationInputError::SummerEndpointMismatch)
            } else {
                Ok(())
            }
        })
}

fn check_ownership(
    observation: &ParticipationObservation<'_>,
) -> Result<(), ParticipationInputError> {
    if let ResultOwnership::Bound {
        performance,
        source_athlete,
        ..
    } = observation.ownership
    {
        if performance != observation.performance {
            return Err(ParticipationInputError::ForeignSubject);
        }
        if source_athlete.id.is_empty()
            || source_athlete.id.len() > 512
            || source_athlete.id.chars().any(char::is_control)
        {
            return Err(ParticipationInputError::InvalidSourceAthlete);
        }
    }
    Ok(())
}

fn proofs<'a>(
    observation: &'a ParticipationObservation<'a>,
    season: InSeasonEvidence<'a>,
) -> impl Iterator<Item = ProofReference<'a>> {
    let ownership = match observation.ownership {
        ResultOwnership::Bound { proof, .. } => Some(proof),
        ResultOwnership::Missing | ResultOwnership::Conflicting => None,
    };
    let cohort = match observation.cohort {
        CohortFact::Resolved { proof, .. } => Some(proof),
        CohortFact::Missing | CohortFact::Conflicting => None,
    };
    ownership
        .into_iter()
        .chain(cohort)
        .chain(observation.contexts.iter().map(|fact| fact.proof))
        .chain(observation.high_school_status.iter().map(|fact| fact.proof))
        .chain(
            observation
                .eligibility_windows
                .iter()
                .map(|fact| fact.boundary_proof),
        )
        .chain(super::in_season::proofs(season))
}

fn check_proofs(
    query: CandidateQuery<'_>,
    observation: &ParticipationObservation<'_>,
    season: InSeasonEvidence<'_>,
) -> Result<(), ParticipationInputError> {
    proofs(observation, season).try_for_each(|proof| {
        if proof.athlete != observation.athlete {
            return Err(ParticipationInputError::ForeignSubject);
        }
        visible(query, proof)?;
        proofs(observation, season).try_for_each(|other| {
            if proof.fact == other.fact && proof != other {
                Err(ParticipationInputError::ConflictingProofIdentity)
            } else {
                Ok(())
            }
        })
    })
}

pub(super) fn visible(
    query: CandidateQuery<'_>,
    proof: ProofReference<'_>,
) -> Result<bool, ParticipationInputError> {
    let visible = proof
        .known
        .visible(query.binding.frozen.lineage, query.binding.frozen.revision)?;
    if visible && proof.acquired_at > query.binding.frozen.captured_at {
        return Err(ParticipationInputError::AcquisitionAfterFrozenInstant);
    }
    Ok(visible)
}
