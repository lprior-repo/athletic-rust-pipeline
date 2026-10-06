use fjall::Keyspace;
use std::collections::HashSet;
use std::path::Path;

use super::keys::{key_label, observation_prefix, view_observation_key};
use super::{Consolidated, Entity, Store, StoreError, StoreResult, StoreStats, Table};

mod directory;
mod identity;
#[cfg(test)]
mod journal_snapshot_tests;
mod snapshot;
mod view;

pub(crate) use directory::directory_bytes;
pub use identity::build_athlete_identity_projection;
pub(crate) use snapshot::sweep_stale_temporaries;
pub use snapshot::{csv_failure, publish_atomically, read_rows, write_snapshot_rows};
pub use view::StoreSnapshot;

impl Store {
    pub(super) fn last_sequence(entities: &Keyspace, table: Table) -> StoreResult<Option<u64>> {
        if table.generation_partitioned() {
            return Ok(None);
        }
        let prefix = observation_prefix(table);
        let mut highest: Option<u64> = None;
        for guard in entities.prefix(prefix.as_slice()) {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            let (_, _, sequence) =
                view_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                    detail: format!("table {} holds a malformed observation key", table.file()),
                })?;
            highest = Some(match highest {
                Some(current) => current.max(sequence),
                None => sequence,
            });
        }
        Ok(highest)
    }

    pub fn for_each_merged<T: Entity>(
        &self,
        table: Table,
        visit: impl FnMut(T) -> StoreResult<()>,
    ) -> StoreResult<u64> {
        self.snapshot().for_each_merged(table, visit)
    }

    pub fn scan<T: Entity>(&self, table: Table) -> StoreResult<Vec<T>> {
        self.snapshot().scan(table)
    }

    pub fn consolidate<T: Entity>(
        &self,
        table: Table,
        out_path: &Path,
    ) -> StoreResult<Consolidated> {
        let result = self.snapshot().consolidate::<T>(table, out_path)?;
        self.flush()?;
        Ok(result)
    }

    pub fn consolidate_table(&self, table: Table, out_path: &Path) -> StoreResult<Consolidated> {
        let result = self.snapshot().consolidate_table(table, out_path)?;
        self.flush()?;
        Ok(result)
    }

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

    pub fn journal_contains(&self, phase: &str, key: &str) -> StoreResult<bool> {
        self.journal
            .contains_key(Self::journal_key(phase, key))
            .map_err(|source| StoreError::Read { source })
    }

    pub fn journal_payload(
        &self,
        phase: &str,
        key: &str,
    ) -> StoreResult<Option<serde_json::Value>> {
        let Some(raw) = self
            .journal
            .get(Self::journal_key(phase, key))
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(None);
        };
        decode_journal_payload(phase, key, raw.as_ref()).map(Some)
    }

    pub fn journal_keys(&self, phase: &str) -> StoreResult<HashSet<String>> {
        let prefix = Self::journal_key(phase, "");
        let mut keys = HashSet::new();
        for guard in self.journal.prefix(prefix.as_slice()) {
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

    pub fn journal_payloads(&self, phase: &str) -> StoreResult<Vec<serde_json::Value>> {
        let prefix = Self::journal_key(phase, "");
        let mut out = Vec::new();
        for guard in self.journal.prefix(prefix.as_slice()) {
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

pub(crate) fn decode_journal_payload(
    phase: &str,
    key: &str,
    raw: &[u8],
) -> StoreResult<serde_json::Value> {
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: format!("{phase}/{key}"),
            source,
        })?;
    value
        .get("payload")
        .cloned()
        .ok_or_else(|| StoreError::Invariant {
            detail: format!("journal entry {phase}/{key} has no payload"),
        })
}
