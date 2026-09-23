//! One-time import of a pre-Fjall store, marker-guarded so an interrupted run finishes on the next open.

use fjall::PersistMode;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use super::keys::{observation_id, observation_key};
use super::{Store, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

/// Rows buffered into one commit while importing a legacy journal: the import once built one batch
/// per table, so a 1.9 GB `athletes.jsonl` meant every row resident at once and an interrupted run
/// restarted from the file's head. Each chunk now commits with the byte offset that follows it, so
/// the ceiling is one chunk and a run resumes at its last commit.
const IMPORT_CHUNK_ROWS: u64 = 20_000;

/// The byte ceiling for the same commit, sized for a journal of unusually fat rows.
const IMPORT_CHUNK_BYTES: u64 = 32 * 1024 * 1024;

/// Marker key written after a table's legacy JSONL journal has been imported.
fn imported_marker(table: Table) -> String {
    format!("imported:{}", table.file())
}

/// Marker key holding the byte offset a table's import has durably committed.
fn offset_marker(table: Table) -> String {
    format!("import_offset:{}", table.file())
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
            let read = read_line(&mut reader, &mut line, path)?;
            if read == 0 {
                break;
            }
            line_no = line_no.saturating_add(1);
            // The offset is a byte count in the store's `u64` ledger. A read length wider than `u64`
            // cannot occur on a 64-bit target; saturating keeps the offset monotone if one could.
            chunk.offset = chunk
                .offset
                .saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
            // `trim_ascii` allocates nothing, so a 1.9 GB journal's line is parsed in place.
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

/// One legacy table's import in flight: the batch being filled, the sequences it has consumed, and
/// the byte offset that follows its last row.
///
/// The chunk is what makes the ceiling a chunk instead of a table — a 1.9 GB journal used to be one
/// batch, every row resident at once, which is what killed the process before its completion marker
/// landed and made the next start begin the file again.
struct ImportChunk<'a> {
    store: &'a Store,
    table: Table,
    batch: fjall::OwnedWriteBatch,
    base: u64,
    rows: u64,
    bytes: u64,
    offset: u64,
}

impl<'a> ImportChunk<'a> {
    fn new(store: &'a Store, table: Table) -> StoreResult<Self> {
        Ok(Self {
            store,
            table,
            batch: store.db.batch(),
            base: store.reserve(table, 0)?,
            rows: 0,
            bytes: 0,
            offset: store.import_offset(table)?,
        })
    }

    /// Validate one journal line, key it, and commit the chunk once it is full.
    fn push(&mut self, body: &[u8], path: &Path, line_no: u64) -> StoreResult<()> {
        if self.rows >= MAX_ROWS_PER_TABLE {
            return Err(StoreError::Legacy {
                detail: format!(
                    "{} line {line_no} exceeds the {MAX_ROWS_PER_TABLE} observation cap",
                    path.display()
                ),
            });
        }
        let id = observation_id(body).map_err(|error| StoreError::Legacy {
            detail: format!("{} line {line_no}: {error}", path.display()),
        })?;
        let key = observation_key(self.table, id, self.base);
        self.batch.insert(&self.store.entities, key, body);
        self.base = self.base.saturating_add(1);
        self.rows = self.rows.saturating_add(1);
        // The chunk ceiling is a `u64` byte count, so the body length is widened once here. A body
        // longer than `u64::MAX` cannot exist on a 64-bit target; saturating keeps the total
        // monotone, which is what makes the ceiling trip rather than wrap.
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(body.len()).unwrap_or(u64::MAX));
        if self.rows >= IMPORT_CHUNK_ROWS || self.bytes >= IMPORT_CHUNK_BYTES {
            self.commit()?;
        }
        Ok(())
    }

    /// Commit the buffered rows together with the offset that follows them, so the offset is exactly
    /// as durable as the rows it describes.
    fn commit(&mut self) -> StoreResult<()> {
        if self.rows == 0 {
            return Ok(());
        }
        self.store.reserve(self.table, self.rows)?;
        // `durability` and `commit` both consume the batch, so the field is swapped for a fresh one
        // and the filled batch is committed by value.
        let mut batch = std::mem::replace(&mut self.batch, self.store.db.batch());
        batch.insert(
            &self.store.meta,
            offset_marker(self.table),
            self.offset.to_string().as_bytes(),
        );
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.rows = 0;
        self.bytes = 0;
        Ok(())
    }
}

/// Read one line, newline included, and report the bytes taken so the caller's offset stays exact.
fn read_line(
    reader: &mut BufReader<std::fs::File>,
    line: &mut Vec<u8>,
    path: &Path,
) -> StoreResult<usize> {
    reader
        .read_until(b'\n', line)
        .map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })
}

impl Store {
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
