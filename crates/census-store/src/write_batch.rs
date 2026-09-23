//! A caller's page of work as one commit: appends across tables, and the journal entries that name
//! them.
//!
//! An adapter that finishes a unit of work writes rows into several tables and then records the unit in
//! the journal, so a resume skips it. Written one call at a time that is one durability boundary per
//! call — the Athletic.net bio flush reaches seventy `fdatasync` calls for a sixty-four-unit page, each
//! of them free to land while its neighbours have not — and a reader between two of them sees half a
//! unit. [`Store::write_batch`] takes the whole page as one batch instead: every table's observations,
//! the marks those reservations move, and the journal entries all reach the database in a single
//! commit, so the page is one `fdatasync` and a reader sees the page or none of it. Counted on this
//! machine over a store test: the same sixty-four-unit page costs seventy `fdatasync` calls written one
//! call at a time, and one written as a batch.
//!
//! The batch validates as it buffers — every record encoded and keyed, every journal entry inside its
//! ceiling — so a refused call leaves the store untouched and a batch that reaches its commit holds
//! only rows the store can read back. The tables' row ceilings are applied to every table before the
//! first reservation moves, so a batch refused for one table's size leaves every counter where it was.
//!
//! Derived state is not carried here: [`Store::replace_many`](crate::Store::replace_many) is a
//! read-modify-write of a table's whole content, and folding that into an evidence page would make one
//! unit's commit depend on every row the table already holds.

use fjall::PersistMode;
use serde::Serialize;

use super::batch::{refuse_over_bound, refuse_over_journal};
use super::keys::{observation_id, observation_key};
use super::{
    Store, StoreError, StoreResult, Table, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
};
use crate::clock::{Clock, SystemClock};

/// One table's buffered page, already encoded and keyed.
struct Page {
    table: Table,
    records: Vec<(String, Vec<u8>)>,
}

/// Appends across tables and the journal entries that name them, committed together.
///
/// Started by [`Store::write_batch`]. Nothing is written until [`StoreBatch::commit`], and a batch
/// dropped without one leaves the store exactly as it found it.
pub struct StoreBatch<'s> {
    store: &'s Store,
    pages: Vec<Page>,
    journal: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Store {
    /// Start a batch: the appends and journal entries buffered on it commit in one call.
    pub fn write_batch(&self) -> StoreBatch<'_> {
        StoreBatch {
            store: self,
            pages: Vec::new(),
            journal: Vec::new(),
        }
    }
}

impl StoreBatch<'_> {
    /// Buffer records for one table.
    ///
    /// Every record is serialized and keyed here, so an unencodable one refuses this call rather than
    /// the commit, and pages for the same table are kept as one.
    pub fn append_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut encoded = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing a batched observation".to_string(),
                source,
            })?;
            encoded.push((observation_id(&value)?.to_string(), value));
        }
        match self.pages.iter_mut().find(|page| page.table == table) {
            Some(page) => page.records.extend(encoded),
            None => self.pages.push(Page {
                table,
                records: encoded,
            }),
        }
        Ok(())
    }

    /// Record a completed unit of work in the same commit as the rows it names.
    ///
    /// The entry's `at` stamp is read when the entry is buffered, not at the commit: the producers
    /// that use this call commit the batch holding both the rows and the entry right after.
    ///
    /// The entry is bounded exactly as [`Store::journal_done`] bounds it, and the refusal names the
    /// phase and key without either reaching the batch.
    pub fn journal_done<T: Serialize>(
        &mut self,
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
        let row_key = Store::journal_key(phase, key);
        refuse_over_journal(phase, key, "key", row_key.len(), MAX_JOURNAL_KEY_BYTES)?;
        refuse_over_journal(phase, key, "value", value.len(), MAX_JOURNAL_VALUE_BYTES)?;
        self.journal.push((row_key, value));
        Ok(())
    }

    /// Whether the batch holds nothing to write.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.journal.is_empty()
    }

    /// Commit every buffered append and journal entry as one batch, one durability boundary.
    ///
    /// The reservation, the batch and the commit are one critical section of the append lock, for the
    /// reason [`Store::append_many`] states: the writer that reserves first commits first, so no batch
    /// can leave a table's mark below observations an earlier batch stored above it.
    pub fn commit(self) -> StoreResult<()> {
        if self.is_empty() {
            return Ok(());
        }
        let store = self.store;
        let _appends = store.lock_appends();
        for page in &self.pages {
            let count =
                u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
            let reached = store
                .sequences
                .next_sequence(page.table)
                .checked_add(count)
                .ok_or(StoreError::CounterOverflow)?;
            refuse_over_bound(page.table, reached)?;
        }
        let mut batch = store.db.batch();
        for page in self.pages {
            let table = page.table;
            let count =
                u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
            let reserved = store.reserve(table, count)?;
            for (offset, (id, value)) in page.records.into_iter().enumerate() {
                let offset = u64::try_from(offset).map_err(|_| StoreError::CounterOverflow)?;
                let sequence = reserved
                    .base
                    .checked_add(offset)
                    .ok_or(StoreError::CounterOverflow)?;
                batch.insert(
                    &store.entities,
                    observation_key(table, &id, sequence),
                    value,
                );
            }
            store.put_mark(&mut batch, table, reserved.mark);
            let rows = store.count(table)?.saturating_add(count);
            store.put_row_mark(&mut batch, table, rows);
        }
        for (key, value) in self.journal {
            batch.insert(&store.journal, key, value);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
