//! Read path: prefix iteration over the entities keyspace, the merging scan, consolidation to
//! JSONL, store statistics and the journal reads that back a resumed run.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use fjall::Keyspace;
use std::collections::{BTreeMap, HashSet};
use std::io::BufRead;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::keys::{key_label, split_observation_key, table_prefix};
use super::{
    Consolidated, Entity, Store, StoreError, StoreResult, StoreStats, Table, MAX_ROWS_PER_TABLE,
};

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
        }
    }

    /// Per-table observation counts and the database footprint.
    pub fn stats(&self) -> StoreResult<StoreStats> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut observations = 0_u64;
        for table in Table::ALL {
            // Per-table counts come from the sequence counters, which are exact: the next free
            // sequence equals the number of observations ever appended to that table.
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

/// Write one JSON object per row, newline-terminated, beside the store; report the mailboxes the
/// merge withheld on the way out. The directory is created here so a fresh store root works.
fn write_snapshot<T: Entity>(path: &Path, rows: &[T]) -> StoreResult<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| io_failure(parent, source))?;
    }
    let file = std::fs::File::create(path).map_err(|source| io_failure(path, source))?;
    let mut writer = BufWriter::new(file);
    let mut withheld = 0_usize;
    for record in rows {
        withheld = withheld.saturating_add(record.withheld_mailboxes());
        serde_json::to_writer(&mut writer, record).map_err(|source| json_failure(path, source))?;
        writer
            .write_all(b"\n")
            .map_err(|source| io_failure(path, source))?;
    }
    writer.flush().map_err(|source| io_failure(path, source))?;
    Ok(withheld)
}

/// A filesystem refusal while writing `path`.
fn io_failure(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// An encode refusal while writing `path`: a row that never encoded has no key to name.
fn json_failure(path: &Path, source: serde_json::Error) -> StoreError {
    StoreError::Json {
        detail: format!("writing {}", path.display()),
        source,
    }
}
/// Read a JSONL snapshot file into typed rows.
///
/// A single unparseable row is tolerated; a second one is a [`StoreError::Json`], because a
/// truncated tail can lose one trailing row and nothing else.
pub fn read_rows<T: for<'de> serde::Deserialize<'de>>(
    path: &std::path::Path,
) -> StoreResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut rows = Vec::new();
    let mut unparseable = 0usize;
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(row) => rows.push(row),
            Err(error) => {
                unparseable = unparseable.saturating_add(1);
                if unparseable > 1 {
                    return Err(StoreError::Json {
                        detail: format!("{}:{}", path.display(), index.saturating_add(1)),
                        source: error,
                    });
                }
            }
        }
    }
    Ok(rows)
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
