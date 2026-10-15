use super::{invalid, Inputs};
use crate::public_identity::context;
use crate::{StoreResult, Table};
use census_domain::model::{
    AthleteCandidateId, CanonicalAthlete, CanonicalPerformance, RelayResult, SourceObservation,
};
use std::collections::BTreeSet;

impl Inputs {
    pub(in crate::public_identity) fn record_keys(
        &self,
        identity: &context::Inputs,
        members: &[AthleteCandidateId],
    ) -> StoreResult<Vec<String>> {
        let mut keys = BTreeSet::new();
        for member in members {
            let athlete = identity
                .athletes
                .iter()
                .find(|row| row.id.as_str() == member.as_str())
                .ok_or_else(|| invalid("manual affected member missing"))?;
            insert(&mut keys, Table::Athletes, athlete.id.as_str())?;
            observations(&mut keys, identity, athlete)?;
        }
        for physical in &self.performances {
            let row: CanonicalPerformance =
                physical.decode(Table::Performances, &physical.key()?)?;
            if owns(members, row.athlete.as_str()) {
                insert(&mut keys, Table::Performances, row.id.as_str())?;
            }
        }
        for physical in &self.relays {
            let row: RelayResult = physical.decode(Table::RelayResults, &physical.key()?)?;
            if row.members.iter().any(|member| {
                member
                    .athlete
                    .as_ref()
                    .is_some_and(|id| owns(members, id.as_str()))
            }) {
                insert(&mut keys, Table::RelayResults, row.id.as_str())?;
            }
        }
        Ok(keys.into_iter().collect())
    }
}

fn observations(
    keys: &mut BTreeSet<String>,
    identity: &context::Inputs,
    athlete: &CanonicalAthlete,
) -> StoreResult<()> {
    identity.observations.iter().try_for_each(|observation| {
        let SourceObservation::Athlete(row) = observation else {
            return Ok(());
        };
        if athlete
            .identities()
            .any(|source| source.namespace == row.namespace && source.id == row.source_athlete_id)
        {
            insert(keys, Table::SourceObservations, observation.id())?;
        }
        Ok(())
    })
}

fn owns(members: &[AthleteCandidateId], id: &str) -> bool {
    members.iter().any(|member| member.as_str() == id)
}

fn insert(keys: &mut BTreeSet<String>, table: Table, id: &str) -> StoreResult<()> {
    if id.is_empty() || id.len() > crate::MAX_ID_BYTES || id.chars().any(char::is_control) {
        return Err(invalid("manual affected logical key"));
    }
    let key = format!("{}:{id}", table.file());
    if !keys.contains(&key) && keys.len() >= 1024 {
        return Err(invalid("manual affected record count bound"));
    }
    keys.insert(key);
    Ok(())
}
