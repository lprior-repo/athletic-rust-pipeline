//! Read path: prefix iteration over the entities keyspace, the merging scan, consolidation to
//! JSONL, store statistics and the journal reads that back a resumed run.
//!
//! `snapshot` holds the file half — the publication every artifact is written through, the JSONL
//! snapshot writers, the sweep of the temporaries a dead writer left behind (its `temporaries`
//! submodule) and the reader that turns one back into typed rows — and `directory` the recursive byte
//! total `stats` reports. This file holds the store half: the `Store` methods a caller reads through.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef, SourceObjectIdentity,
    SourceObservation,
};
use fjall::Keyspace;
use std::collections::HashSet;
use std::path::Path;

use super::keys::{key_label, split_observation_key, table_prefix};
use super::{
    Consolidated, Entity, Store, StoreError, StoreResult, StoreStats, Table, MAX_ROWS_PER_TABLE,
};

mod directory;
mod snapshot;

pub(crate) use directory::directory_bytes;
pub use snapshot::{csv_failure, publish_atomically, read_rows, write_snapshot_rows};
pub(crate) use snapshot::{open_snapshot_writer, sweep_stale_temporaries};

impl Store {
    /// Highest sequence already stored for a table.
    ///
    /// Keys sort by id first and sequence second, so the *last* key in the keyspace does not carry
    /// this table's highest sequence. Resuming from it would hand out sequence numbers that are
    /// already stored for other ids, so this walks the table's prefix and takes the true maximum.
    ///
    /// A mark is where a reopen resumes from now, so this walk has exactly one caller left: the open
    /// that finds no mark for a table — a store written before marks existed — and derives one. See
    /// [`Counters::seeded`](super::sequences::Counters::seeded).
    pub(super) fn last_sequence(entities: &Keyspace, table: Table) -> StoreResult<Option<u64>> {
        let prefix = table_prefix(table);
        let mut highest: Option<u64> = None;
        for guard in entities.prefix(&prefix) {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            let (_, _, sequence) =
                split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                    detail: format!("table {} holds a malformed observation key", table.file()),
                })?;
            highest = Some(match highest {
                Some(current) => current.max(sequence),
                None => sequence,
            });
        }
        Ok(highest)
    }

    /// Visit every observation of a table merged into one entity per id, in id order, without ever
    /// holding more than one id.
    ///
    /// [`Store::scan`] collects this pass. A caller that consumes rows one at a time — a
    /// consolidation writing the snapshot, an export writing a CSV — calls this instead and never
    /// holds the table. Both yield the same rows in the same order: `entities` is keyed
    /// `<table>\0<id>\0<sequence:u64 big-endian>`, so an id's versions are contiguous and the ids
    /// arrive in the byte order a `BTreeMap<String, _>` collected them in before this existed.
    ///
    /// The pass's memory is bounded by one id's versions rather than by the table: the entity under
    /// construction lives until the first row of a different id arrives, and `visit` takes it there.
    /// Everything else the loop holds is one deserialized row and the key its guard borrows, both
    /// dropped before the next row is read, and an unchanged id compares against the entity already
    /// in hand rather than building a `String` for it — the map this replaced allocated one id per
    /// row it read, on top of every entity.
    ///
    /// [`MAX_ROWS_PER_TABLE`] counts every observation read, duplicates included, and the refusal it
    /// raises is the one [`Store::scan`] raises. A refusal from `visit` stops the pass and is
    /// returned as it stands: a snapshot that could not write a row does not keep merging rows nobody
    /// will read.
    pub fn for_each_merged<T: Entity>(
        &self,
        table: Table,
        mut visit: impl FnMut(T) -> StoreResult<()>,
    ) -> StoreResult<u64> {
        let prefix = table_prefix(table);
        let mut merged: Option<(String, T)> = None;
        let mut seen = 0_u64;
        let mut visited = 0_u64;
        for guard in self.entities.prefix(&prefix) {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let record: T =
                serde_json::from_slice(raw.as_ref()).map_err(|source| StoreError::Decode {
                    key: key_label(&key),
                    source,
                })?;
            seen = seen.saturating_add(1);
            if seen > MAX_ROWS_PER_TABLE {
                return Err(StoreError::TooManyRows {
                    table: table.file().to_string(),
                    max: usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX),
                });
            }
            if merged
                .as_ref()
                .is_some_and(|(id, _)| id.as_str() == record.entity_id())
            {
                if let Some((_, entity)) = merged.as_mut() {
                    entity.merge(record);
                }
                continue;
            }
            // A different id: the entity in hand is complete, so it is published, handed over, and
            // the memory it held is the visitor's now. A refused visit returns before the replacement
            // row is read any further.
            if let Some((_, mut entity)) = merged.replace((record.entity_id().to_string(), record))
            {
                entity.publish();
                visit(entity)?;
                visited = visited.saturating_add(1);
            }
        }
        if let Some((_, mut entity)) = merged {
            entity.publish();
            visit(entity)?;
            visited = visited.saturating_add(1);
        }
        Ok(visited)
    }

    /// Every observation of a table, merged into one entity per id and sorted by id.
    ///
    /// This collects [`Store::for_each_merged`]: the rows, their order and its refusals are that
    /// pass's. A caller that does not need the whole table resident should call it directly.
    pub fn scan<T: Entity>(&self, table: Table) -> StoreResult<Vec<T>> {
        let mut rows = Vec::new();
        self.for_each_merged(table, |row| {
            rows.push(row);
            Ok(())
        })?;
        Ok(rows)
    }

    /// Merge a table and write the materialized snapshot as JSONL, the read model every report and
    /// spreadsheet consumes. The withheld count comes out of the same merge pass that writes the
    /// rows, so reporting it never re-scans the table.
    ///
    /// The merge streams: each row is published as [`Store::for_each_merged`] produces it, so the pass
    /// holds one id rather than the table and a twenty-million-row table consolidates in the memory a
    /// one-row one does. A snapshot that cannot write a row stops the merge there and
    /// [`publish_atomically`] discards the staged file, so a refused consolidation publishes nothing
    /// and the previous snapshot stays whole.
    pub fn consolidate<T: Entity>(
        &self,
        table: Table,
        out_path: &Path,
    ) -> StoreResult<Consolidated> {
        let mut rows = 0_usize;
        let mut withheld = 0_usize;
        publish_atomically(out_path, |temporary| {
            let mut writer = open_snapshot_writer(temporary, out_path)?;
            self.for_each_merged::<T>(table, |row| {
                rows = rows.saturating_add(1);
                withheld = withheld.saturating_add(row.withheld_mailboxes());
                writer.push(&row)
            })?;
            writer.finish()
        })?;
        self.flush()?;
        Ok(Consolidated { rows, withheld })
    }

    /// Force the write-ahead journal to disk.
    /// Consolidate one table through the entity type that owns its rows. The table-to-type mapping
    /// lives here, next to the `Entity` impls, so callers stay free of a seven-arm match.
    pub fn consolidate_table(&self, table: Table, out_path: &Path) -> StoreResult<Consolidated> {
        match table {
            Table::Schools => self.consolidate::<CanonicalSchool>(table, out_path),
            Table::Teams => self.consolidate::<CanonicalTeam>(table, out_path),
            Table::Coaches => self.consolidate::<CanonicalCoach>(table, out_path),
            Table::Athletes => self.consolidate::<CanonicalAthlete>(table, out_path),
            Table::Meets => self.consolidate::<CanonicalMeet>(table, out_path),
            Table::Events => self.consolidate::<CanonicalEvent>(table, out_path),
            Table::Performances => self.consolidate::<CanonicalPerformance>(table, out_path),
            Table::SourceIdentities => self.consolidate::<SourceObjectIdentity>(table, out_path),
            Table::Conflicts => self.consolidate::<RetainedConflict>(table, out_path),
            Table::ReviewCases => self.consolidate::<ReviewCase>(table, out_path),
            Table::Coverage => self.consolidate::<CoverageRow>(table, out_path),
            Table::Snapshots => self.consolidate::<CollectionSnapshot>(table, out_path),
            Table::SourceAccess => self.consolidate::<SourceAccessCondition>(table, out_path),
            Table::IdentityVerdicts => self.consolidate::<ReviewVerdictRecord>(table, out_path),
            Table::SourceMeets => self.consolidate::<SourceMeetRef>(table, out_path),
            Table::SourceObservations => self.consolidate::<SourceObservation>(table, out_path),
        }
    }

    /// Per-table rows, the observations each table appended, and the database footprint.
    ///
    /// Rows and appended observations are two different figures: a derived table materializes rows
    /// without appending one, and a reservation a failed batch left behind never becomes a row. The
    /// count comes from the store's own ledger and the appended figure from the table's mode, so
    /// neither is the sequence pointer.
    pub fn stats(&self) -> StoreResult<StoreStats> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut appended = Vec::with_capacity(Table::ALL.len());
        let mut observations = 0_u64;
        for table in Table::ALL {
            let rows = self.count(table)?;
            let appended_rows = table.storage_mode().appended_observations(rows);
            observations = observations.saturating_add(appended_rows);
            tables.push((table.file().to_string(), rows));
            appended.push((table.file().to_string(), appended_rows));
        }
        Ok(StoreStats {
            tables,
            appended,
            observations,
            bytes_on_disk: self.entities.disk_space(),
            store_bytes: directory_bytes(self.root())?,
        })
    }

    /// Keys already processed for a phase — the resume set.
    ///
    /// A key under the phase's prefix that does not decode is corruption, not an absence: dropping it
    /// would report work the store has recorded as work that never happened, and a resumed run would
    /// then repeat it. Every key here is `<phase>\0<key>` with a UTF-8 key, because that is the only
    /// shape [`Store::journal_done`] stores.
    pub fn journal_keys(&self, phase: &str) -> StoreResult<HashSet<String>> {
        let prefix = Self::journal_key(phase, "");
        let mut keys = HashSet::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let bytes: &[u8] = raw.as_ref();
            let suffix =
                bytes
                    .strip_prefix(prefix.as_slice())
                    .ok_or_else(|| StoreError::Invariant {
                        detail: format!(
                            "journal key {} is not under phase {phase}",
                            key_label(bytes)
                        ),
                    })?;
            let key = std::str::from_utf8(suffix).map_err(|_| StoreError::Invariant {
                detail: format!(
                    "journal key {} under phase {phase} is not utf-8",
                    key_label(bytes)
                ),
            })?;
            keys.insert(key.to_string());
        }
        Ok(keys)
    }

    /// All journal payloads for a phase (used to rebuild adapter reports).
    ///
    /// An entry without its `payload` is corruption, not an entry to pass over: the payload is the
    /// record of what the work produced, and a reader that silently omits one rebuilds a report from
    /// less than the store holds. Every entry [`Store::journal_done`] writes carries one, even when
    /// the payload is `null`.
    pub fn journal_payloads(&self, phase: &str) -> StoreResult<Vec<serde_json::Value>> {
        let prefix = Self::journal_key(phase, "");
        let mut out = Vec::new();
        for guard in self.journal.prefix(&prefix) {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let value: serde_json::Value =
                serde_json::from_slice(raw.as_ref()).map_err(|source| StoreError::Decode {
                    key: key_label(&key),
                    source,
                })?;
            let payload = value.get("payload").ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "journal entry {} under phase {phase} has no payload",
                    key_label(&key)
                ),
            })?;
            out.push(payload.clone());
        }
        Ok(out)
    }
}
