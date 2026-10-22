use super::{charge, invalid};
use census_domain::model::{
    serialized_bytes, AthleteId, CanonicalPerformance, EventId, Evidence, MeetId, PerformanceId,
    TeamId,
};
use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PROOFS_PER_ENTITY: usize = 65_536;
const MAX_PROOFS: u64 = 1_000_000;
const MAX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(super) enum Entity<'a> {
    Athlete(&'a AthleteId),
    Performance(&'a PerformanceId),
    Team(&'a TeamId),
    Meet(&'a MeetId),
    Event(&'a EventId),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Athlete,
    Performance,
    Team,
    Meet,
    Event,
}

impl<'a> Entity<'a> {
    fn key(self) -> (Kind, &'a str) {
        match self {
            Self::Athlete(id) => (Kind::Athlete, id.as_str()),
            Self::Performance(id) => (Kind::Performance, id.as_str()),
            Self::Team(id) => (Kind::Team, id.as_str()),
            Self::Meet(id) => (Kind::Meet, id.as_str()),
            Self::Event(id) => (Kind::Event, id.as_str()),
        }
    }
}

#[derive(Default)]
pub(super) struct ResultContexts {
    entities: BTreeMap<Kind, BTreeMap<String, BTreeSet<[u8; 32]>>>,
    proofs: u64,
    bytes: u64,
}

impl ResultContexts {
    pub(super) fn retain_source(
        &mut self,
        performance: &CanonicalPerformance,
        source: &census_crawl::milesplit::VerifiedRetainedResultMetadata,
        notes: &super::note::SourceNotes,
        metadata: &Evidence,
    ) -> StoreResult<()> {
        let owned = fingerprint(&super::support::owned_evidence(source, notes))?;
        let metadata = fingerprint(metadata)?;
        for entity in [
            Entity::Athlete(&performance.athlete),
            Entity::Performance(&performance.id),
            Entity::Team(&performance.team),
            Entity::Event(&performance.event),
        ] {
            self.retain(entity, owned)?;
            self.retain(entity, metadata)?;
        }
        self.retain(Entity::Meet(&performance.meet), metadata)
    }

    pub(super) fn retain_independent(
        &mut self,
        performance: &PerformanceId,
        evidence: &Evidence,
    ) -> StoreResult<()> {
        self.retain(Entity::Performance(performance), fingerprint(evidence)?)
    }

    pub(super) fn contains(&self, entity: Entity<'_>, evidence: &Evidence) -> StoreResult<bool> {
        let (kind, id) = entity.key();
        let digest = fingerprint(evidence)?;
        Ok(self
            .entities
            .get(&kind)
            .and_then(|rows| rows.get(id))
            .is_some_and(|proofs| proofs.contains(&digest)))
    }

    fn retain(&mut self, entity: Entity<'_>, digest: [u8; 32]) -> StoreResult<()> {
        let (kind, id) = entity.key();
        let proofs = self.entities.get(&kind).and_then(|rows| rows.get(id));
        if proofs.is_some_and(|proofs| proofs.contains(&digest)) {
            return Ok(());
        }
        if proofs.is_some_and(|proofs| proofs.len() >= MAX_PROOFS_PER_ENTITY) {
            return Err(invalid(
                "verified result context exceeds 65536 proofs per physical entity",
            ));
        }
        let length = id
            .len()
            .checked_add(33)
            .ok_or(StoreError::CounterOverflow)?;
        let mut proof_count = self.proofs;
        let mut bytes = self.bytes;
        charge(
            &mut proof_count,
            1,
            MAX_PROOFS,
            "verified result context proof count",
        )?;
        charge(
            &mut bytes,
            length,
            MAX_BYTES,
            "verified result context encoded bytes",
        )?;
        self.entities
            .entry(kind)
            .or_default()
            .entry(id.into())
            .or_default()
            .insert(digest);
        self.proofs = proof_count;
        self.bytes = bytes;
        Ok(())
    }
}

fn fingerprint(evidence: &Evidence) -> StoreResult<[u8; 32]> {
    Ok(Sha256::digest(serialized_bytes(evidence)?).into())
}

#[cfg(test)]
mod tests;
