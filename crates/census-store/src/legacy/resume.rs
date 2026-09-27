use fjall::{OwnedWriteBatch, PersistMode};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::super::{Store, StoreError, StoreResult};
use super::cursor::{read_legacy_line, LineEnding};

impl Store {
    pub(super) fn import_legacy_resume_journals(&self) -> StoreResult<()> {
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
            self.import_phase(&entry.path(), &mut batch)?;
        }
        batch.insert(&self.meta, "imported:resume-journals", b"1".as_slice());
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }

    fn import_phase(&self, path: &Path, batch: &mut OwnedWriteBatch) -> StoreResult<()> {
        let Some(phase) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return Err(StoreError::Legacy {
                detail: format!("{} names no phase to key its entries by", path.display()),
            });
        };
        let file = File::open(path).map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut reader = BufReader::new(file);
        self.import_phase_entries(phase, path, &mut reader, batch)
    }

    fn import_phase_entries(
        &self,
        phase: &str,
        path: &Path,
        reader: &mut BufReader<File>,
        batch: &mut OwnedWriteBatch,
    ) -> StoreResult<()> {
        let mut line = Vec::new();
        let mut line_no = 0_u64;
        loop {
            line.clear();
            line_no = line_no.saturating_add(1);
            let Some(ending) = read_legacy_line(reader, &mut line, path, line_no)? else {
                break;
            };
            if ending == LineEnding::Truncated {
                tracing::warn!(
                    path = %path.display(),
                    line = line_no,
                    "stopped at a legacy resume-journal line its writer never finished"
                );
                break;
            }
            let trimmed = line.trim_ascii();
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_slice(trimmed).map_err(|error| StoreError::Legacy {
                    detail: format!("{} line {line_no}: {error}", path.display()),
                })?;
            let key = value
                .get("key")
                .and_then(|key| key.as_str())
                .ok_or_else(|| StoreError::Legacy {
                    detail: format!(
                        "{} line {line_no}: a resume-journal entry carries no key",
                        path.display()
                    ),
                })?;
            batch.insert(&self.journal, Self::journal_key(phase, key), trimmed);
        }
        Ok(())
    }
}
