use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use super::{StorageMode, Store, StoreError, StoreResult, Table};

mod chunk;
mod cursor;
mod resume;

use chunk::ImportChunk;
pub(super) use cursor::read_legacy_line;
use cursor::LineEnding;
pub(super) const MAX_LEGACY_LINE_BYTES: usize = 8 * 1024 * 1024;

fn imported_marker(table: Table) -> String {
    format!("imported:{}", table.file())
}

fn offset_marker(table: Table) -> String {
    format!("import_offset:{}", table.file())
}

fn skipped_marker(table: Table) -> String {
    format!("skipped:{}", table.file())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyImport {
    pub observations: u64,
    pub skipped: u64,
}

impl Store {
    pub fn import_legacy(&self) -> StoreResult<LegacyImport> {
        let _appends = self.lock_appends();
        let mut imported = LegacyImport::default();
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
                let mode = table.storage_mode();
                match mode {
                    StorageMode::ObservationLog => {
                        let count = self.import_observations(table, &path)?;
                        imported.observations = imported.observations.saturating_add(count);
                        tracing::info!(
                            table = table.file(),
                            observations = count,
                            "imported legacy entity journal"
                        );
                    }
                    mode => {
                        imported.skipped = imported.skipped.saturating_add(1);
                        self.meta
                            .insert(skipped_marker(table), path.display().to_string().as_bytes())
                            .map_err(|source| StoreError::Write { source })?;
                        tracing::warn!(
                            table = table.file(),
                            mode = ?mode,
                            path = %path.display(),
                            "left a derived table's legacy journal alone: the derivation pass rebuilds it"
                        );
                    }
                }
            }
            self.meta
                .insert(&marker, b"1".as_slice())
                .map_err(|source| StoreError::Write { source })?;
        }
        self.import_legacy_resume_journals()?;
        if imported.skipped > 0 {
            tracing::warn!(
                skipped = imported.skipped,
                "legacy journals of derived tables were left alone"
            );
        }
        self.flush()?;
        Ok(imported)
    }

    fn import_offset(&self, table: Table) -> StoreResult<u64> {
        let key = offset_marker(table);
        let Some(value) = self
            .meta
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(0);
        };
        std::str::from_utf8(&value)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok())
            .ok_or_else(|| StoreError::Legacy {
                detail: format!("{key} is not a byte offset"),
            })
    }

    fn open_import_reader(
        &self,
        table: Table,
        path: &Path,
    ) -> StoreResult<BufReader<std::fs::File>> {
        let file = std::fs::File::open(path).map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut reader = BufReader::new(file);
        let offset = self.import_offset(table)?;
        if offset > 0 {
            reader
                .seek(SeekFrom::Start(offset))
                .map_err(|source| StoreError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
        Ok(reader)
    }

    fn import_observations(&self, table: Table, path: &Path) -> StoreResult<u64> {
        let mut reader = self.open_import_reader(table, path)?;
        let mut chunk = ImportChunk::new(self, table)?;
        let mut line = Vec::new();
        let mut line_no = 0_u64;
        loop {
            line.clear();
            line_no = line_no.saturating_add(1);
            let Some(ending) = read_legacy_line(&mut reader, &mut line, path, line_no)? else {
                break;
            };
            if ending == LineEnding::Truncated {
                break;
            }
            chunk.offset = chunk
                .offset
                .saturating_add(u64::try_from(line.len()).unwrap_or(u64::MAX));
            let trimmed = line.trim_ascii();
            if trimmed.is_empty() {
                continue;
            }
            chunk.push(trimmed, path, line_no)?;
        }
        let rows = chunk.rows;
        chunk.commit()?;
        Ok(rows)
    }
}
