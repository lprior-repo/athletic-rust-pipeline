use fjall::PersistMode;
use serde::Serialize;

use super::batch::{refuse_over_bound, refuse_over_journal, stage_derived};
use super::keys::{observation_id, observation_key};
use super::{
    generation, Store, StoreError, StoreResult, Table, MAX_JOURNAL_KEY_BYTES,
    MAX_JOURNAL_VALUE_BYTES,
};
use crate::batch::{refuse_derived_append, refuse_observation_replacement};
use crate::clock::{Clock, SystemClock};

impl Store {
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        refuse_derived_append(table)?;
        if records.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        self.sequences.plan(table, count)?;
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing an observation".to_string(),
                source,
            })?;
            let id = observation_id(&value)?;
            encoded.push((id, value));
        }
        self.commit_observations(table, encoded)
    }

    fn commit_observations(
        &self,
        table: Table,
        encoded: Vec<(String, Vec<u8>)>,
    ) -> StoreResult<()> {
        let _appends = self.lock_appends();
        let count = u64::try_from(encoded.len()).map_err(|_| StoreError::CounterOverflow)?;
        let reserved = self.sequences.plan(table, count)?;
        let rows = self
            .count(table)?
            .checked_add(count)
            .ok_or(StoreError::CounterOverflow)?;
        let generation = self.generations.next_entities()?;
        let mut batch = self.db.batch();
        for (offset, (id, value)) in encoded.into_iter().enumerate() {
            let offset = u64::try_from(offset).map_err(|_| StoreError::CounterOverflow)?;
            let sequence = reserved
                .base
                .checked_add(offset)
                .ok_or(StoreError::CounterOverflow)?;
            let key = observation_key(table, &id, sequence);
            batch.insert(&self.entities, key, value);
        }
        self.put_mark(&mut batch, table, reserved.mark);
        self.put_row_mark(&mut batch, table, rows);
        generation::write_entities(&mut batch, &self.meta, generation);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.sequences.publish(table, reserved.mark)?;
        self.generations.publish_entities(generation);
        Ok(())
    }

    pub fn append<T: Serialize>(&self, table: Table, record: &T) -> StoreResult<()> {
        self.append_many(table, std::slice::from_ref(record))
    }

    pub fn replace_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        refuse_observation_replacement(table)?;
        if records.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        refuse_over_bound(table, count)?;
        let _appends = self.lock_appends();
        let held = self.count(table)?;
        let generation = self.generations.next_entities()?;
        let derived = self.derived_generation();
        let mut batch = self.db.batch();
        let staged = stage_derived(&mut batch, &self.entities, table, records, derived)?;
        let rows = held.saturating_add(staged.added);
        self.put_row_mark(&mut batch, table, rows);
        generation::write_entities(&mut batch, &self.meta, generation);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.generations.publish_entities(generation);
        Ok(())
    }

    pub fn replace<T: Serialize>(&self, table: Table, record: &T) -> StoreResult<()> {
        self.replace_many(table, std::slice::from_ref(record))
    }

    pub fn journal_done<T: Serialize>(
        &self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> StoreResult<()> {
        let entry = serde_json::json!({
            "key": key,
            "at": SystemClock.today_iso8601(),
            "payload": payload,
        });
        let value = serde_json::to_vec(&entry).map_err(|source| StoreError::Json {
            detail: "serializing a journal entry".to_string(),
            source,
        })?;
        let row_key = Self::journal_key(phase, key);
        refuse_over_journal(phase, key, "key", row_key.len(), MAX_JOURNAL_KEY_BYTES)?;
        refuse_over_journal(phase, key, "value", value.len(), MAX_JOURNAL_VALUE_BYTES)?;
        let mut batch = self.db.batch();
        batch.insert(&self.journal, row_key, value);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
