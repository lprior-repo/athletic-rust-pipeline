mod inventory;
use inventory::encoded_value;
pub(in crate::public_identity) use inventory::Inventory;

use crate::public_identity::{context::Inputs, PublicIdentityError};
use crate::{StoreError, StoreResult};
use census_domain::model::{
    serialized_digest, AthleteCandidateId, CanonicalAthlete, SourceObservation,
};
use census_domain::public_identity_history::local_adjudication::{
    RetainedClaimV1, RetainedIdentitySectionV1,
};
use serde_json::Value;

pub(super) fn inventory(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
) -> StoreResult<Vec<RetainedClaimV1>> {
    build_inventory(inputs, members, None)
}

pub(in crate::public_identity) fn membership_inventory(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
    projection: &census_domain::public_identity_history::HistoryProjection,
    records: &[census_domain::public_identity_history::HistoryRecord],
) -> StoreResult<Vec<RetainedClaimV1>> {
    if membership_authority(members, projection, records).is_none() {
        return inventory(inputs, members);
    }
    let claims = build_inventory(inputs, members, Some((projection, records)))?;
    append_memberships(claims, members, projection, records)
}

pub(in crate::public_identity) fn append_memberships(
    claims: Vec<RetainedClaimV1>,
    members: &[AthleteCandidateId],
    projection: &census_domain::public_identity_history::HistoryProjection,
    records: &[census_domain::public_identity_history::HistoryRecord],
) -> StoreResult<Vec<RetainedClaimV1>> {
    let mut inventory = Inventory {
        claims,
        remaining: super::MAX_WITNESS_BYTES,
    };
    for claim in &inventory.claims {
        inventory.remaining = inventory
            .remaining
            .checked_sub(crate::public_identity::codec::encode(claim, inventory.remaining)?.len())
            .ok_or(StoreError::CounterOverflow)?;
    }
    for record in records {
        if let census_domain::public_identity_history::HistoryRequest::BindCandidateMembership {
            membership,
            ..
        } = &record.request
        {
            if members.contains(&membership.candidate)
                && projection.membership_claim(&membership.id) == Some(membership)
            {
                let value = encoded_value(membership)?;
                inventory.push(
                    &membership.candidate,
                    RetainedIdentitySectionV1::CandidateMembership,
                    usize::try_from(record.sequence).map_err(|_| StoreError::CounterOverflow)?,
                    &record.digest,
                    membership.id.clone(),
                    &value,
                )?;
            }
        }
    }
    Ok(inventory.claims)
}

pub(in crate::public_identity) type MembershipAuthority<'a> = (
    &'a census_domain::public_identity_history::HistoryProjection,
    &'a [census_domain::public_identity_history::HistoryRecord],
);

pub(in crate::public_identity) fn membership_authority<'a>(
    members: &[AthleteCandidateId],
    projection: &'a census_domain::public_identity_history::HistoryProjection,
    records: &'a [census_domain::public_identity_history::HistoryRecord],
) -> Option<MembershipAuthority<'a>> {
    records.iter().any(|record| matches!(&record.request,
        census_domain::public_identity_history::HistoryRequest::BindCandidateMembership { membership, .. }
        if members.contains(&membership.candidate) && projection.membership_claim(&membership.id).is_some()))
        .then_some((projection, records))
}

fn build_inventory(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
    authority: Option<MembershipAuthority<'_>>,
) -> StoreResult<Vec<RetainedClaimV1>> {
    let mut inventory = Inventory {
        claims: Vec::new(),
        remaining: super::MAX_WITNESS_BYTES,
    };
    let mut primaries = std::collections::BTreeSet::new();
    for member in members {
        let (ordinal, athlete) = inputs
            .athletes
            .iter()
            .enumerate()
            .find(|(_, row)| row.id.as_str() == member.as_str())
            .ok_or(PublicIdentityError::Corrupt("manual member missing"))?;
        if let Some(primary) = &athlete.source {
            let key = serialized_digest(&(&primary.namespace, &primary.id))?;
            if !primaries.insert(key)
                && !explained_primary(inputs.athletes.iter(), members, primary, authority)?
            {
                return Err(PublicIdentityError::Corrupt(
                    "manual correction needs source epoch evidence",
                )
                .into());
            }
        }
        inventory.athlete(member, ordinal, athlete)?;
        for (ordinal, observation) in inputs.observations.iter().enumerate() {
            inventory.observation(member, athlete, ordinal, observation)?;
        }
    }
    Ok(inventory.claims)
}

#[cfg(test)]
mod tests;

pub(in crate::public_identity) fn explained_primary<'a>(
    athletes: impl Iterator<Item = &'a CanonicalAthlete>,
    members: &[AthleteCandidateId],
    primary: &census_domain::model::SourceIdentity,
    authority: Option<MembershipAuthority<'_>>,
) -> StoreResult<bool> {
    let Some((projection, records)) = authority else {
        return Ok(false);
    };
    let group: Vec<_> = athletes
        .filter(|row| {
            members
                .iter()
                .any(|member| row.id.as_str() == member.as_str())
                && row.source.as_ref().is_some_and(|source| {
                    source.namespace == primary.namespace && source.id == primary.id
                })
        })
        .collect();
    let memberships: Vec<_> = records.iter().filter_map(|record| {
        if let census_domain::public_identity_history::HistoryRequest::BindCandidateMembership { membership, .. } = &record.request {
            if projection.membership_claim(&membership.id) == Some(membership) { return Some(membership); }
        }
        None
    }).collect();
    let originals: Vec<_> = group
        .iter()
        .filter(|row| {
            !memberships
                .iter()
                .any(|claim| claim.candidate.as_str() == row.id.as_str())
        })
        .collect();
    let [original] = originals.as_slice() else {
        return Ok(false);
    };
    Ok(group.iter().all(|row| {
        row.id == original.id
            || memberships.iter().any(|claim| {
                claim.candidate.as_str() == row.id.as_str()
                    && claim.source.namespace == primary.namespace
                    && claim.source.identifier == primary.id
                    && projection
                        .binding_claim(&claim.anchor_binding_id)
                        .is_some_and(|anchor| {
                            anchor.candidate.as_str() == original.id.as_str()
                                && anchor.source == claim.source
                        })
            })
    }))
}
