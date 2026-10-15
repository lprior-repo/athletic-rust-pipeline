use super::context::{Inputs, BUDGET};
use super::PublicIdentityError;
use crate::StoreResult;
use census_domain::model::{
    separate_generic_identity_cases, AcceptedAthleteIdentity, AthleteCandidateId,
    AthleteIdentityIndex, IdentityApplication, IdentityProjectionBuilder, ReviewCase,
    SourceIdentity,
};

pub(super) fn accept_inputs(
    inputs: &Inputs,
    candidate: &AthleteCandidateId,
    at: &str,
) -> StoreResult<(AcceptedAthleteIdentity, SourceIdentity)> {
    let (index, bound) = indexed(inputs)?;
    let source = inputs
        .athletes
        .iter()
        .filter(|athlete| athlete.id.as_str() == candidate.as_str())
        .next_back()
        .and_then(|athlete| athlete.source.clone())
        .ok_or(PublicIdentityError::NotSourceBound)?;
    let builder = IdentityProjectionBuilder::new(index, &bound, &inputs.verdicts)?;
    for application in builder.source_applications(at) {
        if let IdentityApplication::Accepted(accepted) = application? {
            if matches!(accepted.record().members.as_slice(), [member] if &member.subject == candidate)
            {
                return Ok((accepted, source));
            }
        }
    }
    Err(PublicIdentityError::NotSourceBound.into())
}

pub(super) fn accept_reviewed(
    inputs: &Inputs,
    case_id: &str,
    at: &str,
) -> StoreResult<AcceptedAthleteIdentity> {
    let (index, bound) = indexed(inputs)?;
    let builder = IdentityProjectionBuilder::new(index, &bound, &inputs.verdicts)?;
    for (case, application) in builder.reviewed_applications(at) {
        if case.id == case_id {
            return match application? {
                IdentityApplication::Accepted(accepted) => Ok(accepted),
                IdentityApplication::Retained(_) => Err(PublicIdentityError::NotReviewed.into()),
            };
        }
    }
    Err(PublicIdentityError::NotReviewed.into())
}

fn indexed(inputs: &Inputs) -> StoreResult<(AthleteIdentityIndex, Vec<ReviewCase>)> {
    let mut athletes = inputs.athletes.clone();
    crate::read::enrich_retained_observations(&mut athletes, &inputs.observations, BUDGET)?;
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        index.observe(&athlete)?;
    }
    let (_, bound) = separate_generic_identity_cases(inputs.cases.clone(), &inputs.verdicts);
    Ok((index, bound))
}

pub(super) fn identity_material(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
) -> StoreResult<String> {
    let (index, _) = indexed(inputs)?;
    let evidence = index.case_evidence("local-adjudication-material", "v1", members)?;
    Ok(ReviewCase::pending_with_evidence(
        census_domain::model::ATHLETE_IDENTITY_FAMILY,
        "local-adjudication-material",
        "local-adjudication-material",
        "v1",
        evidence,
    )
    .id)
}
