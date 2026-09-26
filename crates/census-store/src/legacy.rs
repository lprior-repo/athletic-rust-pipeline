//! One-time import of a pre-Fjall store, marker-guarded so an interrupted run finishes on the next open.

use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use super::{StorageMode, Store, StoreError, StoreResult, Table};

mod chunk;
mod cursor;
mod resume;

use chunk::ImportChunk;
pub(super) use cursor::read_legacy_line;
use cursor::LineEnding;
/// The most one legacy line may occupy: the largest serialized observation the import will hold.
///
/// [`IMPORT_CHUNK_BYTES`] cannot bound a line. It is a budget for a whole commit and is only
/// consulted once a line exists, so a corrupt or hostile multi-gigabyte line would be resident in
/// full before anything refused it. This ceiling is checked while the line is being read, and the
/// bytes past it are never read at all — a line that crosses it is corruption, and a legacy journal
/// that holds one is refused rather than half-imported.
pub(super) const MAX_LEGACY_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Marker key written after a table's legacy JSONL journal has been imported.
fn imported_marker(table: Table) -> String {
    format!("imported:{}", table.file())
}

/// Marker key holding the byte offset a table's import has durably committed.
fn offset_marker(table: Table) -> String {
    format!("import_offset:{}", table.file())
}

/// Marker key recording that a table's legacy journal was left alone because the table holds derived
/// state. Its value is the file that was left alone.
fn skipped_marker(table: Table) -> String {
    format!("skipped:{}", table.file())
}

/// What one [`Store::import_legacy`] call did with the journals it found: the observations it
/// imported, and the journals it left alone because their table is rebuilt by derivation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyImport {
    /// Observations committed from append-only tables' journals.
    pub observations: u64,
    /// Legacy journals refused because their table is not an append-only observation log.
    pub skipped: u64,
}

impl Store {

    /// Import pre-Fjall journals once. A table is marked imported only after every observation of
    /// that table has been committed, so an interrupted import resumes instead of restarting.
    ///
    /// This is not called by [`Store::open`]: opening a store is a read, and this is the one write
    /// that turns a pre-Fjall root into a Fjall one. The paths that have decided to migrate say so by
    /// calling it — the offline census run (`cli::cycle`), the `import-legacy` verb, and the service
    /// bootstrap that owns the store for the live route — and every call is idempotent: the second
    /// one finds the markers and does nothing.
    ///
    /// Only the append-only tables are imported: their journal is evidence, and this import is the
    /// only way evidence an older store holds reaches this one. Every other mode holds derived state
    /// — a read model the derivation pass rebuilds from the merged rows on every pass — so a legacy
    /// file for one of those tables is a stale read model rather than progress worth keeping, and
    /// importing it would write rows the mode's own writer never writes: a derived write keys every
    /// row under sequence zero, while `ImportChunk::push` keys imported rows with the sequences it
    /// is spending. A read merges the later key of an id over the earlier one, so an imported copy
    /// would win over the row the next derivation wrote — and no pass would ever remove it, because
    /// the derivation writes the ids it derived, not the keys something else left behind.
    ///
    /// A skipped journal is recorded, logged with the file it left alone, and counted in the report,
    /// but never deleted: a store has to open. The `imported:<table>` marker is written for a skipped
    /// table too — the one-time import is finished with it — and `skipped:<table>` carries the file
    /// that was left alone, so an operator reading `meta` sees both the decision and its reason.
    pub fn import_legacy(&self) -> StoreResult<LegacyImport> {
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

    /// The byte offset a previous import of `table` durably committed, or zero for a never-imported
    /// file. The offset travels in the same batch as the rows it describes, so it can never claim
    /// more progress than the store holds.
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

    /// Open a table's legacy journal, positioned after everything a previous import committed.
    ///
    /// The offset lives in `meta` and is written in the same batch as the rows before it, so a
    /// resume never re-reads a chunk the store already holds. Legacy journals are append-only —
    /// that is the store's own contract — so an offset committed against a shorter file stays a
    /// line boundary in the same file.
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
