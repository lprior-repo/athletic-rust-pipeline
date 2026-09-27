use fjall::{Keyspace, OwnedWriteBatch, PersistMode};

use super::keys::{split_observation_key, table_prefix, DERIVED_SEQUENCE};
use super::{Store, StoreError, StoreResult, Table};

fn row_mark_key(table: Table) -> String {
    format!("rows:{}", table.file())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableWalk {
    pub rows: u64,
    pub highest_sequence: Option<u64>,
    pub foreign_sequences: u64,
    pub repeated_ids: u64,
}

impl Store {
    pub fn walk_table(&self, table: Table) -> StoreResult<TableWalk> {
        walk_keys(&self.entities, table)
    }

    pub(super) fn count(&self, table: Table) -> StoreResult<u64> {
        match stored_row_mark(&self.meta, table)? {
            Some(rows) => Ok(rows),
            None => count_rows(&self.entities, table),
        }
    }

    pub(super) fn put_row_mark(&self, batch: &mut OwnedWriteBatch, table: Table, rows: u64) {
        batch.insert(&self.meta, row_mark_key(table), rows.to_string().as_bytes());
    }

    pub(super) fn seed_row_marks(&self) -> StoreResult<()> {
        let mut derived: Vec<(Table, u64)> = Vec::new();
        for table in Table::ALL {
            if stored_row_mark(&self.meta, table)?.is_none() {
                derived.push((table, count_rows(&self.entities, table)?));
            }
        }
        if derived.is_empty() {
            return Ok(());
        }
        let mut batch = self.db.batch();
        for (table, rows) in &derived {
            self.put_row_mark(&mut batch, *table, *rows);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}

fn stored_row_mark(meta: &Keyspace, table: Table) -> StoreResult<Option<u64>> {
    let key = row_mark_key(table);
    let Some(value) = meta
        .get(&key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    match std::str::from_utf8(&value)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
    {
        Some(rows) => Ok(Some(rows)),
        None => Err(StoreError::Invariant {
            detail: format!("{key} is not a row count"),
        }),
    }
}

pub(super) fn count_rows(entities: &Keyspace, table: Table) -> StoreResult<u64> {
    Ok(walk_keys(entities, table)?.rows)
}

fn walk_keys(entities: &Keyspace, table: Table) -> StoreResult<TableWalk> {
    let mut walk = TableWalk::default();
    let mut previous: Vec<u8> = Vec::new();
    for guard in entities.prefix(table_prefix(table)) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, sequence) =
            split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} holds a malformed observation key", table.file()),
            })?;
        walk.record(id, sequence, &mut previous);
    }
    Ok(walk)
}

impl TableWalk {
    fn record(&mut self, id: &[u8], sequence: u64, previous: &mut Vec<u8>) {
        self.rows = self.rows.saturating_add(1);
        self.highest_sequence = Some(match self.highest_sequence {
            Some(highest) => highest.max(sequence),
            None => sequence,
        });
        if sequence != DERIVED_SEQUENCE {
            self.foreign_sequences = self.foreign_sequences.saturating_add(1);
        }
        if *previous == id {
            self.repeated_ids = self.repeated_ids.saturating_add(1);
        } else {
            previous.clear();
            previous.extend_from_slice(id);
        }
    }
}
