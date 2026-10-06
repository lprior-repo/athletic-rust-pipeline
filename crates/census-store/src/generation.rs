use std::sync::atomic::{AtomicU64, Ordering};

use fjall::{Keyspace, OwnedWriteBatch, PersistMode};

use super::meta;
use super::{Store, StoreError, StoreResult};

pub(super) const ENTITIES: &str = "generation:entities";
pub(super) const DERIVED_CURRENT: &str = "generation:derived:current";
pub(super) const DERIVED_NEXT: &str = "generation:derived:next";
pub(super) const DERIVED_RECLAIM_FROM: &str = "generation:derived:reclaim_from";

pub(super) struct Generations {
    entities: AtomicU64,
    derived_current: AtomicU64,
}

impl Generations {
    pub(super) fn seeded(meta_keyspace: &Keyspace) -> StoreResult<Self> {
        let entities = required(meta_keyspace, ENTITIES)?;
        let derived_current = required(meta_keyspace, DERIVED_CURRENT)?;
        required(meta_keyspace, DERIVED_NEXT)?;
        required(meta_keyspace, DERIVED_RECLAIM_FROM)?;
        Ok(Self {
            entities: AtomicU64::new(entities),
            derived_current: AtomicU64::new(derived_current),
        })
    }

    pub(super) fn entities(&self) -> u64 {
        self.entities.load(Ordering::Relaxed)
    }

    pub(super) fn derived_current(&self) -> u64 {
        self.derived_current.load(Ordering::Relaxed)
    }

    pub(super) fn next_entities(&self) -> StoreResult<u64> {
        self.entities
            .load(Ordering::Relaxed)
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)
    }

    pub(super) fn publish_entities(&self, generation: u64) {
        self.entities.store(generation, Ordering::Relaxed);
    }

    pub(super) fn publish_derived_current(&self, generation: u64) {
        self.derived_current.store(generation, Ordering::Relaxed);
    }
}

fn required(meta_keyspace: &Keyspace, key: &str) -> StoreResult<u64> {
    meta::get_u64(meta_keyspace, key)?.ok_or_else(|| StoreError::SchemaUnknown {
        detail: format!("{key} is missing from a versioned store"),
    })
}

pub(super) fn write_entities(
    batch: &mut OwnedWriteBatch,
    meta_keyspace: &Keyspace,
    generation: u64,
) {
    meta::put_text(batch, meta_keyspace, ENTITIES, &generation.to_string());
}

pub(super) fn write_derived_current(
    batch: &mut OwnedWriteBatch,
    meta_keyspace: &Keyspace,
    generation: u64,
) {
    meta::put_text(
        batch,
        meta_keyspace,
        DERIVED_CURRENT,
        &generation.to_string(),
    );
}

pub(super) fn reserve_derived(db: &fjall::Database, meta_keyspace: &Keyspace) -> StoreResult<u64> {
    let generation = required(meta_keyspace, DERIVED_NEXT)?;
    let next = generation
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let mut batch = db.batch();
    meta::put_text(&mut batch, meta_keyspace, DERIVED_NEXT, &next.to_string());
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(generation)
}

pub(super) fn write_seed(
    batch: &mut OwnedWriteBatch,
    meta_keyspace: &Keyspace,
    entities: u64,
    derived_current: u64,
    derived_next: u64,
    reclaim_from: u64,
) {
    write_entities(batch, meta_keyspace, entities);
    write_derived_current(batch, meta_keyspace, derived_current);
    meta::put_text(
        batch,
        meta_keyspace,
        DERIVED_NEXT,
        &derived_next.to_string(),
    );
    meta::put_text(
        batch,
        meta_keyspace,
        DERIVED_RECLAIM_FROM,
        &reclaim_from.to_string(),
    );
}

impl Store {
    pub fn evidence_generation(&self) -> u64 {
        self.generations.entities()
    }

    pub fn derived_generation(&self) -> u64 {
        self.generations.derived_current()
    }
}
