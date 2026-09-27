use fjall::PersistMode;
use std::path::Path;

use super::super::keys::{observation_id, observation_key};
use super::super::{Store, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};
use super::offset_marker;

const IMPORT_CHUNK_ROWS: u64 = 20_000;

const IMPORT_CHUNK_BYTES: u64 = 32 * 1024 * 1024;

pub(super) struct ImportChunk<'a> {
    store: &'a Store,
    table: Table,
    batch: fjall::OwnedWriteBatch,
    base: u64,
    pub(super) rows: u64,
    bytes: u64,
    pub(super) offset: u64,
    table_rows: u64,
    row_count: u64,
}

impl<'a> ImportChunk<'a> {
    pub(super) fn new(store: &'a Store, table: Table) -> StoreResult<Self> {
        let base = store.sequences.plan(table, 0)?.base;
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

    pub(super) fn push(&mut self, body: &[u8], path: &Path, line_no: u64) -> StoreResult<()> {
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
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(body.len()).unwrap_or(u64::MAX));
        if self.rows >= IMPORT_CHUNK_ROWS || self.bytes >= IMPORT_CHUNK_BYTES {
            self.commit()?;
        }
        Ok(())
    }

    pub(super) fn commit(&mut self) -> StoreResult<()> {
        if self.rows == 0 {
            return Ok(());
        }
        let mark = self.store.sequences.plan(self.table, self.rows)?.mark;
        let mut batch = std::mem::replace(&mut self.batch, self.store.db.batch());
        batch.insert(
            &self.store.meta,
            offset_marker(self.table),
            self.offset.to_string().as_bytes(),
        );
        self.store.put_mark(&mut batch, self.table, mark);
        let rows = self
            .row_count
            .checked_add(self.rows)
            .ok_or(StoreError::CounterOverflow)?;
        self.store.put_row_mark(&mut batch, self.table, rows);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.store.sequences.publish(self.table, mark)?;
        self.row_count = rows;
        self.rows = 0;
        self.bytes = 0;
        Ok(())
    }
}
