use fjall::{Keyspace, OwnedWriteBatch, PersistMode};

use super::keys::{
    derived_generation_prefix, view_derived_key, view_observation_key, Layout, DERIVED_SEQUENCE,
};
use super::{meta, Store, StoreError, StoreResult, Table};

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
        walk_keys(&self.entities, self.layout(), table)
    }

    pub(super) fn layout(&self) -> Layout {
        Layout {
            derived_generation: self.generations.derived_current(),
        }
    }

    pub(super) fn count(&self, table: Table) -> StoreResult<u64> {
        match meta::get_u64(&self.meta, &row_mark_key(table))? {
            Some(rows) => Ok(rows),
            None => count_rows(&self.entities, self.layout(), table),
        }
    }

    pub(super) fn put_row_mark(&self, batch: &mut OwnedWriteBatch, table: Table, rows: u64) {
        put_row_mark(batch, &self.meta, table, rows);
    }

    pub(super) fn seed_row_marks(&self) -> StoreResult<()> {
        let mut derived: Vec<(Table, u64)> = Vec::new();
        for table in Table::ALL {
            if meta::get_u64(&self.meta, &row_mark_key(table))?.is_none() {
                derived.push((table, count_rows(&self.entities, self.layout(), table)?));
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

pub(super) fn put_row_mark(
    batch: &mut OwnedWriteBatch,
    meta_keyspace: &Keyspace,
    table: Table,
    rows: u64,
) {
    meta::put_text(
        batch,
        meta_keyspace,
        &row_mark_key(table),
        &rows.to_string(),
    );
}

pub(super) fn count_rows(entities: &Keyspace, layout: Layout, table: Table) -> StoreResult<u64> {
    Ok(walk_keys(entities, layout, table)?.rows)
}

pub(super) fn count_generation(
    entities: &Keyspace,
    table: Table,
    generation: u64,
) -> StoreResult<u64> {
    let prefix = derived_generation_prefix(table, generation);
    let mut count = 0_u64;
    for guard in entities.prefix(prefix.as_slice()) {
        guard.key().map_err(|source| StoreError::Read { source })?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(count)
}

pub(super) fn walk_keys(
    entities: &Keyspace,
    layout: Layout,
    table: Table,
) -> StoreResult<TableWalk> {
    let prefix = layout.prefix(table);
    let mut walk = TableWalk::default();
    let mut previous: Vec<u8> = Vec::new();
    for guard in entities.prefix(prefix.as_slice()) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if table.generation_partitioned() {
            let (_, id) = view_derived_key(table, &key).ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "table {} holds a malformed derived key {}",
                    table.file(),
                    super::keys::key_label(&key)
                ),
            })?;
            walk.record(id, DERIVED_SEQUENCE, &mut previous);
            continue;
        }
        let (_, id, sequence) =
            view_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "table {} holds a malformed observation key {}",
                    table.file(),
                    super::keys::key_label(&key)
                ),
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
