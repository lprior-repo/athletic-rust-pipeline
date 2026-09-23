//! One legacy table's import in flight: the batch being filled, the sequences it has consumed, and
//! the byte offset that follows its last row.

use fjall::PersistMode;
use std::path::Path;

use super::super::keys::{observation_id, observation_key};
use super::super::{Store, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};
use super::offset_marker;

/// Rows buffered into one commit while importing a legacy journal: the import once built one batch
/// per table, so a 1.9 GB `athletes.jsonl` meant every row resident at once and an interrupted run
/// restarted from the file's head. Each chunk now commits with the byte offset that follows it, so
/// the ceiling is one chunk and a run resumes at its last commit.
const IMPORT_CHUNK_ROWS: u64 = 20_000;

/// The byte ceiling for the same commit, sized for a journal of unusually fat rows.
const IMPORT_CHUNK_BYTES: u64 = 32 * 1024 * 1024;

/// One legacy table's import in flight: the batch being filled, the sequences it has consumed, and
/// the byte offset that follows its last row.
///
/// The chunk is what makes the ceiling a chunk instead of a table — a 1.9 GB journal used to be one
/// batch, every row resident at once, which is what killed the process before its completion marker
/// landed and made the next start begin the file again.
pub(super) struct ImportChunk<'a> {
    store: &'a Store,
    table: Table,
    batch: fjall::OwnedWriteBatch,
    base: u64,
    pub(super) rows: u64,
    bytes: u64,
    pub(super) offset: u64,
    /// The table's rows as the row ceiling counts them: what it held when this chunk opened — the
    /// sequence the zero-length probe reported — plus every row this run has accepted, committed or
    /// still buffered.
    ///
    /// [`ImportChunk::rows`] cannot do this job. It is reset at every chunk boundary, so a ceiling
    /// applied to it bounds a batch rather than a table, which is how a 30-million-row journal
    /// imported past a 20-million-row ceiling.
    table_rows: u64,
    /// The rows the table held when this run opened, from the ledger rather than the sequence
    /// pointer: what the write path counts. Each commit carries `row_count + chunk rows` as the
    /// table's count, because a resumed import commits after the open that seeded the ledger and a
    /// count nobody moves would leave the table holding more rows than it reports.
    row_count: u64,
}

impl<'a> ImportChunk<'a> {
    pub(super) fn new(store: &'a Store, table: Table) -> StoreResult<Self> {
        // The import's zero-length probe: it learns the sequence its rows will be keyed from
        // without reserving any, and the reservation happens when the chunk commits. That sequence
        // is also the number of observations the table already holds — the quantity the append
        // funnel's own ceiling counts — so it is worth holding for the whole run.
        let base = store.reserve(table, 0)?.base;
        Ok(Self {
            store,
            table,
            batch: store.db.batch(),
            base,
            rows: 0,
            bytes: 0,
            offset: store.import_offset(table)?,
            table_rows: base,
            row_count: store.count(table)?,
        })
    }

    /// Validate one journal line, key it, and commit the chunk once it is full.
    pub(super) fn push(&mut self, body: &[u8], path: &Path, line_no: u64) -> StoreResult<()> {
        // The ceiling belongs to the table, not to the chunk: a table that already holds it refuses
        // the import at the row that would cross it, and the refusal names the file and the line to
        // repair. The append funnel refuses the same total at `Store::reserve`, one commit later and
        // with neither the file nor the line an operator needs to repair a legacy journal.
        if self.table_rows >= MAX_ROWS_PER_TABLE {
            return Err(StoreError::Legacy {
                detail: format!(
                    "{} line {line_no}: table {} already holds {} observations, \
                     and the next would pass the {MAX_ROWS_PER_TABLE} row ceiling",
                    path.display(),
                    self.table.file(),
                    self.table_rows
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
        self.table_rows = self.table_rows.saturating_add(1);
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
    /// as durable as the rows it describes, and with the table's new mark, because this batch is the
    /// one that spends the sequences those rows are keyed under.
    pub(super) fn commit(&mut self) -> StoreResult<()> {
        if self.rows == 0 {
            return Ok(());
        }
        // The mark is arithmetic on the reservation, and the reservation is taken here, under the
        // same lock an appender reserves under: an import beside a live writer cannot leave the
        // table's mark below rows that are already stored.
        let _appends = self.store.lock_appends();
        let mark = self.store.reserve(self.table, self.rows)?.mark;
        // `durability` and `commit` both consume the batch, so the field is swapped for a fresh one
        // and the filled batch is committed by value.
        let mut batch = std::mem::replace(&mut self.batch, self.store.db.batch());
        batch.insert(
            &self.store.meta,
            offset_marker(self.table),
            self.offset.to_string().as_bytes(),
        );
        self.store.put_mark(&mut batch, self.table, mark);
        let rows = self.row_count.saturating_add(self.rows);
        self.store.put_row_mark(&mut batch, self.table, rows);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.row_count = rows;
        self.rows = 0;
        self.bytes = 0;
        Ok(())
    }
}
