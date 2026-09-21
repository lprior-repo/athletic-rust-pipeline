//! Read path: prefix iteration over the entities keyspace, the merging scan, consolidation to
//! JSONL, store statistics and the journal reads that back a resumed run.

use anyhow::{bail, Context, Result};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use fjall::Keyspace;
use std::collections::{BTreeMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::Ordering;

use super::keys::{split_observation_key, table_prefix};
use super::{Consolidated, Entity, Store, StoreStats, Table, MAX_ROWS_PER_TABLE};

impl Store {
    /// Highest sequence already stored for a table.
    ///
    /// Keys sort by id first and sequence second, so the *last* key in the keyspace does not carry
    /// this table's highest sequence. Resuming from it would hand out sequence numbers that are
    /// already stored for other ids, so this walks the table's prefix and takes the true maximum.
    pub(super) fn last_sequence(entities: &Keyspace, table: Table) -> Result<Option<u64>> {
        let prefix = table_prefix(table);
        let mut highest: Option<u64> = None;
        for guard in entities.prefix(&prefix) {
            let key = guard.key().context("reading an entity key")?;
            let (_, sequence) = split_observation_key(&key).with_context(|| {
                format!("table {} holds a malformed observation key", table.file())
            })?;
            highest = Some(match highest {
                Some(current) => current.max(sequence),
                None => sequence,
            });
        }
        Ok(highest)
    }

    /// Every observation of a table, merged into one entity per id and sorted by id.
    pub fn scan<T: Entity>(&self, table: Table) -> Result<Vec<T>> {
        let prefix = table_prefix(table);
        let mut merged: BTreeMap<String, T> = BTreeMap::new();
        let mut seen = 0_u64;
        for guard in self.entities.prefix(&prefix) {
            let raw = guard.value().context("reading an observation")?;
            let record: T = serde_json::from_slice(raw.as_ref())
                .with_context(|| format!("parsing an observation of {}", table.file()))?;
            seen = seen.saturating_add(1);
            if seen > MAX_ROWS_PER_TABLE {
                bail!(
                    "{} holds more than {MAX_ROWS_PER_TABLE} observations; consolidate with a smaller window",
                    table.file()
                );
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
    pub fn consolidate<T: Entity>(&self, table: Table, out_path: &Path) -> Result<Consolidated> {
        let rows = self.scan::<T>(table)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let file = std::fs::File::create(out_path)
            .with_context(|| format!("creating {}", out_path.display()))?;
        let mut writer = BufWriter::new(file);
        let mut withheld = 0_usize;
        for record in &rows {
            withheld = withheld.saturating_add(record.withheld_mailboxes());
            serde_json::to_writer(&mut writer, record)
                .with_context(|| format!("writing {}", out_path.display()))?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        self.flush()?;
        Ok(Consolidated {
            rows: rows.len(),
            withheld,
        })
    }

    /// Force the write-ahead journal to disk.
    /// Consolidate one table through the entity type that owns its rows. The table-to-type mapping
    /// lives here, next to the `Entity` impls, so callers stay free of a seven-arm match.
    pub fn consolidate_table(&self, table: Table, out_path: &Path) -> Result<Consolidated> {
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
    pub fn stats(&self) -> Result<StoreStats> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut observations = 0_u64;
        for table in Table::ALL {
            // Per-table counts come from the sequence counters, which are exact: the next free
            // sequence equals the number of observations ever appended to that table.
            let next = self
                .sequences
                .get(table.file())
                .map(|counter| counter.load(Ordering::Relaxed))
                .unwrap_or(0);
            observations = observations.saturating_add(next);
            tables.push((table.file().to_string(), next));
        }
        Ok(StoreStats {
            tables,
            observations,
            bytes_on_disk: self.entities.disk_space(),
        })
    }

    /// Keys already processed for a phase — the resume set.
    pub fn journal_keys(&self, phase: &str) -> Result<HashSet<String>> {
        let prefix = Self::journal_key(phase, "");
        let mut keys = HashSet::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.key().context("reading a journal key")?;
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
    pub fn journal_payloads(&self, phase: &str) -> Result<Vec<serde_json::Value>> {
        let prefix = Self::journal_key(phase, "");
        let mut out = Vec::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.value().context("reading a journal entry")?;
            let value: serde_json::Value = serde_json::from_slice(raw.as_ref())
                .with_context(|| format!("parsing a journal entry of {phase}"))?;
            if let Some(payload) = value.get("payload") {
                out.push(payload.clone());
            }
        }
        Ok(out)
    }
}
