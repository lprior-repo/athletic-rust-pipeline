use super::{charge, invalid};
use census_domain::model::{
    serialized_bytes, AthleteId, CanonicalPerformance, EventId, Evidence, MeetId, PerformanceId,
    TeamId,
};
use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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
        if proofs.is_some_and(|proofs| proofs.len() >= 256) {
            return Err(invalid(
                "verified result context exceeds 256 proofs per physical entity",
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
            1_000_000,
            "verified result context proof count",
        )?;
        charge(
            &mut bytes,
            length,
            64 * 1024 * 1024,
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
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn aggregate_boundary_accepts_duplicate_but_refusal_preserves_result_state() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let added = u64::try_from(athlete.as_str().len() + 33)?;
        let mut context = ResultContexts {
            proofs: 999_999,
            bytes: 64 * 1024 * 1024 - added,
            ..Default::default()
        };
        context.retain(Entity::Athlete(&athlete), [0; 32])?;
        let before = context.entities.clone();
        context.retain(Entity::Athlete(&athlete), [0; 32])?;
        let other = AthleteId::mint("ath_subject", &["foreign"]);
        assert!(matches!(
            context.retain(Entity::Athlete(&other), [0; 32]),
            Err(StoreError::BudgetExceeded {
                what: "verified result context proof count",
                needed: 1_000_001,
                available: 1_000_000
            })
        ));
        assert!(context.entities == before);
        assert_eq!((context.proofs, context.bytes), (1_000_000, 67_108_864));
        Ok(())
    }

    #[test]
    fn byte_refusal_does_not_insert_result_entity_or_charge_count() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let added = u64::try_from(athlete.as_str().len() + 33)?;
        let mut context = ResultContexts {
            proofs: 7,
            bytes: 64 * 1024 * 1024,
            ..Default::default()
        };
        assert!(matches!(context.retain(Entity::Athlete(&athlete), [0; 32]),
            Err(StoreError::BudgetExceeded {
                what: "verified result context encoded bytes", needed, available: 67_108_864
            }) if needed == 67_108_864 + added));
        assert!(context.entities.is_empty());
        assert_eq!((context.proofs, context.bytes), (7, 67_108_864));
        Ok(())
    }

    #[test]
    fn physical_entity_limit_preserves_result_state_and_duplicate_identity() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let mut context = ResultContexts::default();
        for ordinal in 0..=u8::MAX {
            context.retain(Entity::Athlete(&athlete), [ordinal; 32])?;
        }
        let before = context.entities.clone();
        let counts = (context.proofs, context.bytes);
        context.retain(Entity::Athlete(&athlete), [0; 32])?;
        let mut extra = [0; 32];
        extra[1] = 1;
        assert!(matches!(context.retain(Entity::Athlete(&athlete), extra),
            Err(StoreError::Refused { detail })
            if detail == "verified result context exceeds 256 proofs per physical entity"));
        assert!(context.entities == before);
        assert_eq!((context.proofs, context.bytes), counts);
        Ok(())
    }
}
