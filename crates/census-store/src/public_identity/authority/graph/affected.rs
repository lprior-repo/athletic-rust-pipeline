use super::*;
use crate::public_identity::local_adjudication::affected::KeyInventory;
use census_domain::model::{CanonicalPerformance, RelayResult};

pub(in crate::public_identity) fn record_keys<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    members: &[AthleteCandidateId],
) -> StoreResult<Vec<String>> {
    let rows = member_rows(view, members)?;
    admission::affected(view)?;
    let mut keys = KeyInventory::new();
    for member in members {
        let (_, athlete) = rows
            .iter()
            .find(|(_, row)| row.id.as_str() == member.as_str())
            .ok_or(PublicIdentityError::Corrupt(
                "manual affected member missing",
            ))?;
        keys.athlete(athlete)?;
        view.for_each_observation::<SourceObservation>(
            AuthorityTableV2::SourceObservations,
            BUDGET,
            |row, _| keys.observation(athlete, &row),
        )?;
    }
    view.for_each_observation::<CanonicalPerformance>(
        AuthorityTableV2::Performances,
        BUDGET,
        |row, _| keys.performance(members, &row),
    )?;
    view.for_each_observation::<RelayResult>(AuthorityTableV2::RelayResults, BUDGET, |row, _| {
        keys.relay(members, &row)
    })?;
    Ok(keys.finish())
}
