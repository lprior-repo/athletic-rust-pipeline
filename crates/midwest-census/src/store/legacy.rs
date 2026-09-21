//! One-time import of a pre-Fjall store: the per-table entity logs and the resume-journal
//! directory. The import is marker-guarded, so an interrupted run finishes on the next open.

use fjall::PersistMode;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::keys::{observation_id, observation_key};
use super::{Store, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

/// Marker key written after a table's legacy JSONL journal has been imported.
fn imported_marker(table: Table) -> String {
    format!("imported:{}", table.file())
}

impl Store {
    // -- legacy import ----------------------------------------------------------------------------

    /// Import pre-Fjall journals once. A table is marked imported only after every observation of
    /// that table has been committed, so an interrupted import resumes instead of restarting.
    pub(super) fn import_legacy(&self) -> StoreResult<()> {
        for table in Table::ALL {
            let marker = imported_marker(table);
            if self
                .meta
                .contains_key(&marker)
                .map_err(|source| StoreError::Read { source })?
            {
                continue;
            }
            let path = self.table_path(table);
            if path.exists() {
                let count = self.import_observations(table, &path)?;
                tracing::info!(
                    table = table.file(),
                    observations = count,
                    "imported legacy entity journal"
                );
            }
            self.meta
                .insert(&marker, b"1".as_slice())
                .map_err(|source| StoreError::Write { source })?;
        }
        self.import_legacy_resume_journals()?;
        self.flush()
    }

    fn import_observations(&self, table: Table, path: &Path) -> StoreResult<u64> {
        let file = std::fs::File::open(path).map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut batch = self.db.batch();
        let mut count = 0_u64;
        let mut base = self.reserve(table, 0)?;
        for (line_no, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|source| StoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let line_number = line_no.saturating_add(1);
            if count >= MAX_ROWS_PER_TABLE {
                return Err(StoreError::Legacy {
                    detail: format!(
                        "{} line {} exceeds the {MAX_ROWS_PER_TABLE} observation cap",
                        path.display(),
                        line_number
                    ),
                });
            }
            let bytes = trimmed.as_bytes();
            let id = observation_id(bytes).map_err(|error| StoreError::Legacy {
                detail: format!("{} line {}: {error}", path.display(), line_number),
            })?;
            let key = observation_key(table, id, base);
            batch.insert(&self.entities, key, bytes);
            base = base.saturating_add(1);
            count = count.saturating_add(1);
        }
        self.reserve(table, count)?;
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(count)
    }

    fn import_legacy_resume_journals(&self) -> StoreResult<()> {
        let dir = self.root.join("journal");
        if !dir.exists() {
            return Ok(());
        }
        if self
            .meta
            .contains_key("imported:resume-journals")
            .map_err(|source| StoreError::Read { source })?
        {
            return Ok(());
        }
        let entries = std::fs::read_dir(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let mut batch = self.db.batch();
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
            let path = entry.path();
            let Some(phase) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let file = std::fs::File::open(&path).map_err(|source| StoreError::Io {
                path: path.clone(),
                source,
            })?;
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|source| StoreError::Io {
                    path: path.clone(),
                    source,
                })?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue;
                };
                let Some(key) = value.get("key").and_then(|key| key.as_str()) else {
                    continue;
                };
                batch.insert(
                    &self.journal,
                    Self::journal_key(phase, key),
                    trimmed.as_bytes(),
                );
            }
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.meta
            .insert("imported:resume-journals", b"1".as_slice())
            .map_err(|source| StoreError::Write { source })
    }
}
