use super::*;

#[derive(Serialize)]
pub(super) struct Group<'a> {
    pub(super) schema: &'static str,
    pub(super) store: &'a StoreIdentity,
    pub(super) qualification: &'static str,
    pub(super) public_id: &'a PublicAthleteId,
    pub(super) representative: &'a AthleteCandidateId,
    pub(super) established: EstablishmentOrdinal,
    pub(super) claims: &'a [Claim<'a>],
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "claim", rename_all = "snake_case")]
pub(super) enum Claim<'a> {
    Binding(&'a BindingClaim),
    Membership(&'a CandidateMembershipClaimV1),
}

pub(super) fn current<'a>(
    accepted: &'a AcceptedPublicIdentitySnapshot,
    target: &PublicAthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CurrentTarget<'a>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
    {
        return Err(Error::TargetMoved.into());
    }
    let PublicResolution::Canonical(resolved) = accepted.resolve_public(target)? else {
        return Err(Error::TargetMoved.into());
    };
    if resolved.public_id != *target {
        return Err(Error::TargetMoved.into());
    }
    let claims = claims(accepted, target, grant)?;
    let wire = Group {
        schema: "retained-evidence-subject-attachment-group/v1",
        store: accepted.store_identity(),
        qualification: "retained_authority_graph_v3",
        public_id: &resolved.public_id,
        representative: &resolved.representative_candidate,
        established: resolved.established,
        claims: &claims,
    };
    let group = encoding::object(&wire, grant)?;
    Ok(CurrentTarget {
        accepted,
        resolved,
        group,
    })
}

fn claims<'a>(
    accepted: &'a AcceptedPublicIdentitySnapshot,
    target: &PublicAthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Claim<'a>>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let ids = accepted.component_bindings(target)?;
    if ids.len() > ReplayLimits::SMALL.members_per_operation {
        return Err(Error::Corrupt("canonical target component exceeds member bound").into());
    }
    let mut claims = Vec::new();
    claims
        .try_reserve_exact(ids.len())
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    for id in &ids {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let claim = match (accepted.binding_claim(id), accepted.membership_claim(id)) {
            (Some(binding), None) if binding.id == *id => Claim::Binding(binding),
            (None, Some(membership)) if membership.id == *id => Claim::Membership(membership),
            _ => return Err(Error::Corrupt("canonical target claim absent or ambiguous").into()),
        };
        claims.push(claim);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests;
