//! Write path: reserve observation sequences, append validated batches, record journal entries.

use anyhow::{Context, Result};
use fjall::PersistMode;
use serde::Serialize;
use std::sync::atomic::Ordering;

use super::keys::{observation_id, observation_key};
use super::{Store, Table};

impl Store {
    /// Reserve `count` consecutive observation sequences for a table.
    pub(super) fn reserve(&self, table: Table, count: u64) -> Result<u64> {
        let counter = self
            .sequences
            .get(table.file())
            .context("table has no sequence counter")?;
        let start = counter.fetch_add(count, Ordering::Relaxed);
        Ok(start)
    }

    /// Append observations to a table. Each observation is its own row, exactly like the JSONL
    /// journals: merging happens at read time, so an entity seen twice keeps both evidence sets.
    ///
    /// Every record is validated before a single sequence is reserved, so a rejected batch leaves
    /// both the keyspace and the sequence counters untouched.
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).context("serializing an observation")?;
            let id = observation_id(&value)?.to_string();
            encoded.push((id, value));
        }
        let count = u64::try_from(encoded.len()).context("record count does not fit u64")?;
        let base = self.reserve(table, count)?;
        let mut batch = self.db.batch();
        for (offset, (id, value)) in encoded.into_iter().enumerate() {
            let offset = u64::try_from(offset).context("record offset does not fit u64")?;
            let sequence = base
                .checked_add(offset)
                .context("observation sequence overflowed")?;
            let key = observation_key(table, &id, sequence);
            batch.insert(&self.entities, key, value);
        }
        batch
            // `SyncData` is one `fdatasync` per batch: a crash cannot lose a completed append, and a
            // batch is a whole adapter page, not a single row.
            .durability(Some(PersistMode::SyncData))
            .commit()
            .with_context(|| {
                format!(
                    "committing {} observations to {}",
                    records.len(),
                    table.file()
                )
            })
    }

    pub fn append<T: Serialize>(&self, table: Table, record: &T) -> Result<()> {
        self.append_many(table, std::slice::from_ref(record))
    }

    /// Record that a unit of work completed. Doubles as the resume ledger.
    pub fn journal_done<T: Serialize>(&self, phase: &str, key: &str, payload: &T) -> Result<()> {
        let entry = serde_json::json!({
            "key": key,
            "at": crate::net::now_iso8601(),
            "payload": payload,
        });
        let value = serde_json::to_vec(&entry).context("serializing a journal entry")?;
        let mut batch = self.db.batch();
        batch.insert(&self.journal, Self::journal_key(phase, key), value);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .with_context(|| format!("committing journal entry {phase}:{key}"))
    }
}
