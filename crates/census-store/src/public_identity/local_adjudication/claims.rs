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
    if !records.iter().any(|record| matches!(&record.request,
        census_domain::public_identity_history::HistoryRequest::BindCandidateMembership { membership, .. }
        if members.contains(&membership.candidate) && projection.membership_claim(&membership.id).is_some())) {
        return inventory(inputs, members);
    }
    let claims = build_inventory(inputs, members, Some((projection, records)))?;
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

type MembershipAuthority<'a> = (
    &'a census_domain::public_identity_history::HistoryProjection,
    &'a [census_domain::public_identity_history::HistoryRecord],
);

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
            if !primaries.insert(key) && !explained_primary(inputs, members, primary, authority)? {
                return Err(PublicIdentityError::Corrupt(
                    "manual correction needs source epoch evidence",
                )
                .into());
            }
        }
        inventory.athlete(member, ordinal, athlete)?;
        for (ordinal, observation) in inputs.observations.iter().enumerate() {
            let SourceObservation::Athlete(row) = observation else {
                continue;
            };
            if athlete.identities().any(|source| {
                source.namespace == row.namespace && source.id == row.source_athlete_id
            }) {
                let value = encoded_value(observation)?;
                inventory.push(
                    member,
                    RetainedIdentitySectionV1::SourceObservation,
                    ordinal,
                    &serialized_digest(observation)?,
                    String::new(),
                    &value,
                )?;
            }
        }
    }
    Ok(inventory.claims)
}

struct Inventory {
    claims: Vec<RetainedClaimV1>,
    remaining: usize,
}

impl Inventory {
    fn athlete(
        &mut self,
        member: &AthleteCandidateId,
        ordinal: usize,
        athlete: &CanonicalAthlete,
    ) -> StoreResult<()> {
        let value = encoded_value(athlete)?;
        let row_digest = serialized_digest(athlete)?;
        let fields = value
            .as_object()
            .ok_or(PublicIdentityError::Corrupt("manual athlete record"))?;
        for (field, value) in fields {
            match value {
                Value::Array(values) if !values.is_empty() => {
                    for (index, value) in values.iter().enumerate() {
                        self.push(
                            member,
                            RetainedIdentitySectionV1::Athlete,
                            ordinal,
                            &row_digest,
                            format!("/{field}/{index}"),
                            value,
                        )?;
                    }
                }
                _ => self.push(
                    member,
                    RetainedIdentitySectionV1::Athlete,
                    ordinal,
                    &row_digest,
                    format!("/{field}"),
                    value,
                )?,
            }
        }
        Ok(())
    }

    fn push(
        &mut self,
        candidate: &AthleteCandidateId,
        section: RetainedIdentitySectionV1,
        ordinal: usize,
        row_digest: &str,
        claim_path: String,
        value: &Value,
    ) -> StoreResult<()> {
        if self.claims.len() >= 4096 {
            return Err(
                PublicIdentityError::Corrupt("manual claim inventory exceeds bound").into(),
            );
        }
        let claim = RetainedClaimV1 {
            candidate: candidate.clone(),
            section,
            ordinal: u64::try_from(ordinal).map_err(|_| StoreError::CounterOverflow)?,
            row_digest: row_digest.to_owned(),
            claim_path,
            claim_digest: serialized_digest(value)?,
            retained_value: value.clone(),
        };
        let encoded = crate::public_identity::codec::encode(&claim, self.remaining)?;
        self.remaining = self
            .remaining
            .checked_sub(encoded.len())
            .ok_or(StoreError::CounterOverflow)?;
        self.claims
            .try_reserve(1)
            .map_err(|error| StoreError::Refused {
                detail: format!("manual claim inventory allocation: {error}"),
            })?;
        self.claims.push(claim);
        Ok(())
    }
}

fn encoded_value<T: serde::Serialize>(row: &T) -> StoreResult<Value> {
    serde_json::to_value(row).map_err(|source| StoreError::Json {
        detail: "encoding retained manual claim".into(),
        source,
    })
}

#[cfg(test)]
mod tests;

fn explained_primary(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
    primary: &census_domain::model::SourceIdentity,
    authority: Option<MembershipAuthority<'_>>,
) -> StoreResult<bool> {
    let Some((projection, records)) = authority else {
        return Ok(false);
    };
    let group: Vec<_> = inputs
        .athletes
        .iter()
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
