use super::context::{Inputs, BUDGET};
use super::PublicIdentityError;
use crate::StoreResult;
use census_domain::model::{
    separate_generic_identity_cases, AcceptedAthleteIdentity, AthleteCandidateId,
    AthleteIdentityIndex, IdentityApplication, IdentityProjectionBuilder, SourceIdentity,
};

pub(super) fn accept_inputs(
    inputs: &Inputs,
    candidate: &AthleteCandidateId,
    at: &str,
) -> StoreResult<(AcceptedAthleteIdentity, SourceIdentity)> {
    let mut athletes = inputs.athletes.clone();
    let mut index = AthleteIdentityIndex::default();
    let mut source = None;
    crate::read::enrich_retained_observations(&mut athletes, &inputs.observations, BUDGET)?;
    for athlete in athletes {
        index.observe(&athlete)?;
        if athlete.id.as_str() == candidate.as_str() {
            source = athlete.source;
        }
    }
    let source = source.ok_or(PublicIdentityError::NotSourceBound)?;
    let cases = inputs.cases.clone();
    let verdicts = &inputs.verdicts;
    let (_, bound) = separate_generic_identity_cases(cases, verdicts);
    let builder = IdentityProjectionBuilder::new(index, &bound, verdicts)?;
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
