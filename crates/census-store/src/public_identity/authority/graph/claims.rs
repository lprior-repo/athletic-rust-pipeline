use super::*;
use crate::public_identity::local_adjudication::claims::{self, Inventory};
use census_domain::model::serialized_digest;
use census_domain::public_identity_history::local_adjudication::RetainedClaimV1;
use census_domain::public_identity_history::{HistoryProjection, HistoryRecord};
use std::collections::BTreeSet;

pub(in crate::public_identity) fn membership_inventory<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    members: &[AthleteCandidateId],
    projection: &HistoryProjection,
    records: &[HistoryRecord],
) -> StoreResult<Vec<RetainedClaimV1>> {
    let rows = member_rows(view, members)?;
    let authority = claims::membership_authority(members, projection, records);
    let mut inventory = Inventory {
        claims: Vec::new(),
        remaining: crate::public_identity::local_adjudication::MAX_WITNESS_BYTES,
    };
    let mut primaries = BTreeSet::new();
    for member in members {
        let (ordinal, athlete) = rows
            .iter()
            .find(|(_, row)| row.id.as_str() == member.as_str())
            .ok_or(PublicIdentityError::Corrupt("manual member missing"))?;
        if let Some(primary) = &athlete.source {
            let key = serialized_digest(&(&primary.namespace, &primary.id))?;
            if !primaries.insert(key)
                && !claims::explained_primary(
                    rows.iter().map(|(_, row)| row),
                    members,
                    primary,
                    authority,
                )?
            {
                return Err(PublicIdentityError::Corrupt(
                    "manual correction needs source epoch evidence",
                )
                .into());
            }
        }
        inventory.athlete(member, *ordinal, athlete)?;
        let mut ordinal = 0_usize;
        view.for_each_observation::<SourceObservation>(
            AuthorityTableV2::SourceObservations,
            BUDGET,
            |row, _| {
                let current = ordinal;
                ordinal = ordinal.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                inventory.observation(member, athlete, current, &row)
            },
        )?;
    }
    if authority.is_some() {
        claims::append_memberships(inventory.claims, members, projection, records)
    } else {
        Ok(inventory.claims)
    }
}
