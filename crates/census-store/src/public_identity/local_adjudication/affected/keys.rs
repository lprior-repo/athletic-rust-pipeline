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
        let mut keys = KeyInventory::new();
        for member in members {
            let athlete = identity
                .athletes
                .iter()
                .find(|row| row.id.as_str() == member.as_str())
                .ok_or_else(|| invalid("manual affected member missing"))?;
            keys.athlete(athlete)?;
            for observation in &identity.observations {
                keys.observation(athlete, observation)?;
            }
        }
        for physical in &self.performances {
            let row = physical.decode(Table::Performances, &physical.key()?)?;
            keys.performance(members, &row)?;
        }
        for physical in &self.relays {
            let row = physical.decode(Table::RelayResults, &physical.key()?)?;
            keys.relay(members, &row)?;
        }
        Ok(keys.finish())
    }
}

pub(in crate::public_identity) struct KeyInventory {
    keys: BTreeSet<String>,
}

impl KeyInventory {
    pub(in crate::public_identity) fn new() -> Self {
        Self {
            keys: BTreeSet::new(),
        }
    }

    pub(in crate::public_identity) fn athlete(
        &mut self,
        athlete: &CanonicalAthlete,
    ) -> StoreResult<()> {
        insert(&mut self.keys, Table::Athletes, athlete.id.as_str())
    }

    pub(in crate::public_identity) fn observation(
        &mut self,
        athlete: &CanonicalAthlete,
        observation: &SourceObservation,
    ) -> StoreResult<()> {
        let SourceObservation::Athlete(row) = observation else {
            return Ok(());
        };
        if athlete
            .identities()
            .any(|source| source.namespace == row.namespace && source.id == row.source_athlete_id)
        {
            insert(&mut self.keys, Table::SourceObservations, observation.id())?;
        }
        Ok(())
    }

    pub(in crate::public_identity) fn performance(
        &mut self,
        members: &[AthleteCandidateId],
        row: &CanonicalPerformance,
    ) -> StoreResult<()> {
        if owns(members, row.athlete.as_str()) {
            insert(&mut self.keys, Table::Performances, row.id.as_str())?;
        }
        Ok(())
    }

    pub(in crate::public_identity) fn relay(
        &mut self,
        members: &[AthleteCandidateId],
        row: &RelayResult,
    ) -> StoreResult<()> {
        if row.members.iter().any(|member| {
            member
                .athlete
                .as_ref()
                .is_some_and(|id| owns(members, id.as_str()))
        }) {
            insert(&mut self.keys, Table::RelayResults, row.id.as_str())?;
        }
        Ok(())
    }

    pub(in crate::public_identity) fn finish(self) -> Vec<String> {
        self.keys.into_iter().collect()
    }
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
