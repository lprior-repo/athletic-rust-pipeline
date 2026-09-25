//! Write path: reserve observation sequences, append validated batches, record journal entries.

use fjall::PersistMode;
use serde::Serialize;

use super::batch::{drop_unnamed, refuse_over_bound, refuse_over_journal, stage_derived};
use super::keys::{observation_id, observation_key};
use super::sequences::Reserved;
use super::{
    StorageMode, Store, StoreError, StoreResult, Table, MAX_JOURNAL_KEY_BYTES,
    MAX_JOURNAL_VALUE_BYTES,
};
use crate::clock::{Clock, SystemClock};

impl Store {
    /// Reserve `count` consecutive observation sequences for a table, with the mark the reserving
    /// batch has to store beside its observations.
    ///
    /// This is the one funnel every reservation passes through — the appenders, the legacy import's
    /// zero-length probe and its chunk commits alike — so the sequence ceiling is applied here, on
    /// the sequence the reservation would reach, rather than once per call site. Every caller that
    /// reserves more than zero holds the append lock, so two batches can never both find room and
    /// then take it: no counter a store holds can pass [`MAX_ROWS_PER_TABLE`], and a refused
    /// reservation leaves its counter exactly where it was. A zero-length probe is always allowed,
    /// because it reaches nothing.
    pub(super) fn reserve(&self, table: Table, count: u64) -> StoreResult<Reserved> {
        let reached = self
            .sequences
            .next_sequence(table)
            .checked_add(count)
            .ok_or(StoreError::CounterOverflow)?;
        refuse_over_bound(table, reached)?;
        self.sequences.reserve(table, count)
    }

    /// Append observations to a table. Each observation is its own row, exactly like the JSONL
    /// journals: merging happens at read time, so an entity seen twice keeps both evidence sets. The
    /// batch carries the table's new mark beside those rows, so the next open resumes at the sequence
    /// this batch ended at instead of walking the table to find it.
    ///
    /// The ceiling is a bound on the sequence space: the batch would reach `base + count`, so a
    /// batch that would take the table past `crate::MAX_ROWS_PER_TABLE` is refused before the reservation
    /// moves the counter, and a refused batch leaves the counter where it found it.
    ///
    /// Every record is validated before a single sequence is reserved, so a rejected batch leaves
    /// both the keyspace and the sequence counters untouched.
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        let would_reach = self
            .sequences
            .next_sequence(table)
            .checked_add(count)
            .ok_or(StoreError::CounterOverflow)?;
        refuse_over_bound(table, would_reach)?;
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing an observation".to_string(),
                source,
            })?;
            let id = observation_id(&value)?.to_string();
            encoded.push((id, value));
        }
        self.commit_observations(table, encoded)
    }

    /// Commit one page of encoded observations, with the table's mark in the same batch.
    ///
    /// The reservation, the batch and the commit are one critical section of the append lock. That is
    /// what makes the mark a high-water mark: the writer that reserves first also commits first, so no
    /// batch can leave a mark below observations a batch already stored above it — a reopen in that
    /// state would hand out sequences the database holds.
    ///
    /// Nothing in the section can fail on a record: the caller encoded and validated every one of
    /// them, so the only fallible work left is the reservation's arithmetic and the commit.
    fn commit_observations(
        &self,
        table: Table,
        encoded: Vec<(String, Vec<u8>)>,
    ) -> StoreResult<()> {
        let _appends = self.lock_appends();
        let count = u64::try_from(encoded.len()).map_err(|_| StoreError::CounterOverflow)?;
        let reserved = self.reserve(table, count)?;
        // The rows this batch leaves behind: the count the table's ledger holds — a reservation is not
        // a row, so the sequence mark is never that figure — plus the observations about to be written.
        let rows = self.count(table)?.saturating_add(count);
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
    /// place: the table cannot grow with the number of passes, and [`Store::stats`] keeps counting
    /// what was *appended* — evidence — rather than what was merely re-derived.
    ///
    /// The ceiling binds the batch rather than a sequence, because a batch is a whole row set here:
    /// one call may not hand the table more rows than a scan of what it just wrote would agree to
    /// read, and an over-bound batch is refused before anything is written.
    ///
    /// The row count travels in the same batch as the rows, because a derived write is the only thing
    /// that moves a derived table's size: the count it stores is the rows the batch leaves behind —
    /// what the table held plus the ids this batch is the first to name, or, for a table whose batch is
    /// its whole new content, the ids the batch names and nothing else.
    ///
    /// The append lock orders the batches that move a table's count, exactly as it orders the ones
    /// that move its sequence mark: a count is read and written back, and two writers that both read
    /// the count the store held before either committed would leave the ledger holding one of them.
    ///
    /// Like [`Store::append_many`], every record is validated before the batch is built, so a
    /// rejected record leaves the keyspace untouched.
    pub fn replace_many<T: Serialize>(&self, table: Table, records: &[T]) -> StoreResult<()> {
        // A snapshot write is the table's whole content, so an empty one states that the derivation
        // found nothing and the table must come out empty: otherwise the rows of the previous pass
        // outlive the findings they describe. Every other mode keeps the rows a write does not name,
        // so an empty batch there is a no-op and stays one.
        if records.is_empty() && table.storage_mode() != StorageMode::DerivedSnapshot {
            return Ok(());
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        refuse_over_bound(table, count)?;
        let _appends = self.lock_appends();
        let held = self.count(table)?;
        let mut batch = self.db.batch();
        let staged = stage_derived(&mut batch, &self.entities, table, records)?;
        let rows = match table.storage_mode() {
            // A snapshot write is the table's whole content: the rows it does not name are gone, so
            // what the batch names is all the table holds, however many rows stood there before.
            StorageMode::DerivedSnapshot => {
                drop_unnamed(&self.entities, &mut batch, table, &staged.named)?;
                staged.named_count()?
            }
            StorageMode::DerivedMap | StorageMode::ObservationLog => {
                held.saturating_add(staged.added)
            }
        };
        self.put_row_mark(&mut batch, table, rows);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }

    pub fn replace<T: Serialize>(&self, table: Table, record: &T) -> StoreResult<()> {
        self.replace_many(table, std::slice::from_ref(record))
    }

    /// Record that a unit of work completed. Doubles as the resume ledger.
    ///
    /// The key and the serialized entry are bounded, because the journal is a ledger an operator reads
    /// and a key nobody can print or an entry too large to hold is not a record worth storing. The
    /// value's true size is only known once it is serialized, so both checks run between the encoding
    /// and the batch: the refusal names the phase and the key, and the entry writes nothing at all.
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
