//! Write path: reserve observation sequences, append validated batches, record journal entries.

use fjall::PersistMode;
use serde::Serialize;

use super::keys::{observation_id, observation_key};
use super::{Store, StoreError, StoreResult, Table};
use crate::clock::{Clock, SystemClock};

/// The sequence a derived row is keyed under. Derived state keeps exactly one row per entity id, so
/// the sequence is a constant rather than a reserved observation number.
const DERIVED_SEQUENCE: u64 = 0;

impl Store {
    /// Reserve `count` consecutive observation sequences for a table.
    pub(super) fn reserve(&self, table: Table, count: u64) -> StoreResult<u64> {
        self.sequences.reserve(table, count)
    }

    /// Append observations to a table. Each observation is its own row, exactly like the JSONL
    /// journals: merging happens at read time, so an entity seen twice keeps both evidence sets.
    ///
    /// Every record is validated before a single sequence is reserved, so a rejected batch leaves
    /// both the keyspace and the sequence counters untouched.
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing an observation".to_string(),
                source,
            })?;
            let id = observation_id(&value)?.to_string();
            encoded.push((id, value));
        }
        let count = u64::try_from(encoded.len()).map_err(|_| StoreError::CounterOverflow)?;
        let base = self.reserve(table, count)?;
        let mut batch = self.db.batch();
        for (offset, (id, value)) in encoded.into_iter().enumerate() {
            let offset = u64::try_from(offset).map_err(|_| StoreError::CounterOverflow)?;
            let sequence = base
                .checked_add(offset)
                .ok_or(StoreError::CounterOverflow)?;
            let key = observation_key(table, &id, sequence);
            batch.insert(&self.entities, key, value);
        }
        batch
            // `SyncData` is one `fdatasync` per batch: a crash cannot lose a completed append, and a
            // batch is a whole adapter page, not a single row.
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }

    pub fn append<T: Serialize>(&self, table: Table, record: &T) -> StoreResult<()> {
        self.append_many(table, std::slice::from_ref(record))
    }

    /// Write derived state: one row per entity id, replacing whatever stood in that key.
    ///
    /// Derived rows are a function of the store as it is now, not evidence about a moment in it, so
    /// re-deriving must leave one row per key rather than one observation per pass. The row is keyed
    /// with a fixed sequence of zero and no sequence is reserved, so a later pass overwrites it in
    /// place: the table cannot grow with the number of passes, `MAX_ROWS_PER_TABLE` cannot be
    /// exhausted by re-running a derivation, and [`Store::stats`] keeps counting what was *appended*
    /// — evidence — rather than what was merely re-derived.
    ///
    /// Like [`Store::append_many`], every record is validated before the batch is built, so a
    /// rejected record leaves the keyspace untouched.
    pub fn replace_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut batch = self.db.batch();
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing derived state".to_string(),
                source,
            })?;
            let id = observation_id(&value)?.to_string();
            let key = observation_key(table, &id, DERIVED_SEQUENCE);
            batch.insert(&self.entities, key, value);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }

    pub fn replace<T: Serialize>(&self, table: Table, record: &T) -> StoreResult<()> {
        self.replace_many(table, std::slice::from_ref(record))
    }

    /// Record that a unit of work completed. Doubles as the resume ledger.
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
        let mut batch = self.db.batch();
        batch.insert(&self.journal, Self::journal_key(phase, key), value);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
