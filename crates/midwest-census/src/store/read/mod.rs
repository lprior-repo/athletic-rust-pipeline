//! Read path: prefix iteration over the entities keyspace, the merging scan, consolidation to
//! JSONL, store statistics and the journal reads that back a resumed run.
//!
//! `snapshot` holds the file half — the JSONL snapshot writers, the sweep of the temporaries a dead
//! writer left behind, and the reader that turns one back into typed rows. This file holds the store
//! half: the `Store` methods a caller reads through, and the directory size `stats` reports.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef, SourceObjectIdentity,
};
use fjall::Keyspace;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use super::keys::{key_label, split_observation_key, table_prefix};
use super::{
    Consolidated, Entity, Store, StoreError, StoreResult, StoreStats, Table, MAX_ROWS_PER_TABLE,
};

mod snapshot;

pub use snapshot::read_rows;
pub(in crate::store) use snapshot::{sweep_stale_temporaries, write_snapshot};

impl Store {
    /// Highest sequence already stored for a table.
    ///
    /// Keys sort by id first and sequence second, so the *last* key in the keyspace does not carry
    /// this table's highest sequence. Resuming from it would hand out sequence numbers that are
    /// already stored for other ids, so this walks the table's prefix and takes the true maximum.
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

    /// Every observation of a table, merged into one entity per id and sorted by id.
    pub fn scan<T: Entity>(&self, table: Table) -> StoreResult<Vec<T>> {
        let prefix = table_prefix(table);
        let mut merged: BTreeMap<String, T> = BTreeMap::new();
        let mut seen = 0_u64;
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
            let id = record.entity_id().to_string();
            match merged.get_mut(&id) {
                Some(existing) => existing.merge(record),
                None => {
                    merged.insert(id, record);
                }
            }
        }
        Ok(merged
            .into_values()
            .map(|mut entity| {
                entity.publish();
                entity
            })
            .collect())
    }

    /// Merge a table and write the materialized snapshot as JSONL, the read model every report and
    /// spreadsheet consumes. The withheld count comes out of the same merge pass that writes the
    /// rows, so reporting it never re-scans the table.
    pub fn consolidate<T: Entity>(
        &self,
        table: Table,
        out_path: &Path,
    ) -> StoreResult<Consolidated> {
        let rows = self.scan::<T>(table)?;
        let withheld = write_snapshot(out_path, &rows)?;
        self.flush()?;
        Ok(Consolidated {
            rows: rows.len(),
            withheld,
        })
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
        }
    }

    /// Per-table observation counts and the database footprint.
    pub fn stats(&self) -> StoreResult<StoreStats> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut observations = 0_u64;
        for table in Table::ALL {
            // Per-table counts come from the sequence counters, which are exact: for a table written
            // through `append_many` the next free sequence equals the number of observations ever
            // appended, and for a table written through `replace_many` no sequence is reserved, so
            // the counter a reopen seeds from the highest stored key reports that table's row count.
            let next = self.sequences.appended(table);
            observations = observations.saturating_add(next);
            tables.push((table.file().to_string(), next));
        }
        Ok(StoreStats {
            tables,
            observations,
            bytes_on_disk: self.entities.disk_space(),
            store_bytes: directory_bytes(self.root()),
        })
    }

    /// Keys already processed for a phase — the resume set.
    pub fn journal_keys(&self, phase: &str) -> StoreResult<HashSet<String>> {
        let prefix = Self::journal_key(phase, "");
        let mut keys = HashSet::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let bytes: &[u8] = raw.as_ref();
            if let Some(suffix) = bytes.strip_prefix(prefix.as_slice()) {
                if let Ok(key) = std::str::from_utf8(suffix) {
                    keys.insert(key.to_string());
                }
            }
        }
        Ok(keys)
    }

    /// All journal payloads for a phase (used to rebuild adapter reports).
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
            if let Some(payload) = value.get("payload") {
                out.push(payload.clone());
            }
        }
        Ok(out)
    }
}

/// Recursive byte total of a directory, ignoring entries that cannot be read.
///
/// This is the number to size a copy or a backup by. fjall's own `disk_space()` counts LSM-tree
/// level sizes, so a store whose newest batch still lives in the write-ahead journal reports a
/// figure far below what a copy has to carry — a drill measured `bytes_on_disk 0` on a store whose
/// directory held 229 KB, 223 KB of it journal.
fn directory_bytes(root: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(metadata) if metadata.is_dir() => directory_bytes(&entry.path()),
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        })
        .fold(0_u64, u64::saturating_add)
}
