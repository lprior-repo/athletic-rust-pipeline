use crate::public_identity::{codec, context::Inputs, PublicIdentityError};
use crate::StoreResult;
use census_domain::model::{
    serialized_digest, AcceptedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId,
    CanonicalAthlete, StoreIdentity,
};
use census_domain::public_identity_history::{
    BindingResolution, CandidateDecisionReference, CandidateMembershipClaimV1, HistoryProjection,
    HistoryRequest, PublicAthleteId, PublicResolution, ReplayLimits,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCandidateMembershipRequestV1 {
    pub case_id: String,
    #[serde(deserialize_with = "public_id")]
    pub owner: PublicAthleteId,
    pub anchor_binding_id: String,
    pub candidate: AthleteCandidateId,
}

pub(super) fn digest(
    identity: &StoreIdentity,
    operation: &str,
    request: &SourceCandidateMembershipRequestV1,
) -> StoreResult<String> {
    for text in [
        operation,
        request.case_id.as_str(),
        request.anchor_binding_id.as_str(),
    ] {
        if text.is_empty()
            || text.len() > ReplayLimits::SMALL.text_bytes
            || text.chars().any(char::is_control)
        {
            return Err(PublicIdentityError::OperationConflict.into());
        }
    }
    if operation.len() > 256 {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    Ok(serialized_digest(&(
        "public-identity-source-membership-request-v1",
        identity,
        operation,
        codec::encode(request, 65_536)?,
    ))?)
}

pub(super) fn authorized(
    request: &SourceCandidateMembershipRequestV1,
    digest: &str,
    inputs: &Inputs,
    accepted: &AcceptedAthleteIdentity,
    projection: &HistoryProjection,
) -> StoreResult<HistoryRequest> {
    if accepted.record().kind != AppliedIdentityKind::SamePerson
        || accepted.record().case_id.as_deref() != Some(request.case_id.as_str())
    {
        return Err(PublicIdentityError::NotReviewed.into());
    }
    if !matches!(
        projection
            .resolve_candidate(&request.candidate)
            .map_err(PublicIdentityError::from)?,
        PublicResolution::Unknown
    ) {
        return Err(PublicIdentityError::AlreadyBound.into());
    }
    let anchor = projection
        .binding_claim(&request.anchor_binding_id)
        .ok_or(PublicIdentityError::NotSourceBound)?;
    authorize_anchor(request, projection, anchor)?;
    let first = physical(inputs, &anchor.candidate)?;
    let second = physical(inputs, &request.candidate)?;
    let primary = first
        .source
        .as_ref()
        .ok_or(PublicIdentityError::NotSourceBound)?;
    let other = second
        .source
        .as_ref()
        .ok_or(PublicIdentityError::NotSourceBound)?;
    if primary.namespace != anchor.source.namespace
        || primary.id != anchor.source.identifier
        || other.namespace != primary.namespace
        || other.id != primary.id
        || first.grad_year != second.grad_year
        || !census_domain::model::genders_compatible(first.gender, second.gender)
    {
        return Err(PublicIdentityError::NotSourceBound.into());
    }
    let evidence = CandidateDecisionReference::from_accepted(accepted, ReplayLimits::SMALL)
        .map_err(PublicIdentityError::from)?;
    Ok(HistoryRequest::BindCandidateMembership {
        owner: request.owner.clone(),
        membership: CandidateMembershipClaimV1 {
            id: format!("source_membership_v1_{digest}"),
            candidate: request.candidate.clone(),
            anchor_binding_id: request.anchor_binding_id.clone(),
            source: anchor.source.clone(),
        },
        evidence,
    })
}

fn physical<'a>(
    inputs: &'a Inputs,
    candidate: &AthleteCandidateId,
) -> StoreResult<&'a CanonicalAthlete> {
    let mut matches = inputs
        .athletes
        .iter()
        .filter(|athlete| athlete.id.as_str() == candidate.as_str());
    let athlete = matches.next().ok_or(PublicIdentityError::NotSourceBound)?;
    let source = athlete
        .source
        .as_ref()
        .ok_or(PublicIdentityError::NotSourceBound)?;
    if matches.next().is_some()
        || !athlete.retained_conflicts.is_empty()
        || CanonicalAthlete::mint(
            &athlete.school,
            &athlete.canonical_name,
            athlete.grad_year,
            athlete.gender,
            source,
        ) != athlete.id
    {
        return Err(PublicIdentityError::NotSourceBound.into());
    }
    Ok(athlete)
}

fn public_id<'de, D: serde::Deserializer<'de>>(parser: D) -> Result<PublicAthleteId, D::Error> {
    let raw = String::deserialize(parser)?;
    PublicAthleteId::parse(&raw).map_err(serde::de::Error::custom)
}

fn authorize_anchor(
    request: &SourceCandidateMembershipRequestV1,
    projection: &HistoryProjection,
    anchor: &census_domain::public_identity_history::BindingClaim,
) -> StoreResult<()> {
    match projection
        .resolve_binding(&anchor.id)
        .map_err(PublicIdentityError::from)?
    {
        BindingResolution::Resolved {
            identity:
                PublicResolution::Canonical(owner)
                | PublicResolution::Redirect {
                    successor: owner, ..
                }
                | PublicResolution::RevokedRedirect {
                    successor: owner, ..
                },
            superseded_by: None,
            ..
        } if owner.public_id == request.owner => {}
        _ => return Err(PublicIdentityError::NotSourceBound.into()),
    }
    Ok(())
}
