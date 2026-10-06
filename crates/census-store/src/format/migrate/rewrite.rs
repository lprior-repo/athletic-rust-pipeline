use fjall::{Database, Keyspace, OwnedWriteBatch, PersistMode};

use super::super::{
    CREATED_AT, CREATED_BY, KEY_FORMAT, KEY_FORMAT_VERSION, MIGRATING_TO, STORE_SCHEMA_VERSION,
    STORE_VERSION,
};
use super::Rewritten;
use crate::clock::{Clock, SystemClock};
use crate::keys::{
    derived_generation_prefix, derived_key, key_label, observation_key, observation_prefix,
    view_observation_key, DERIVED_SEQUENCE,
};
use crate::rows::put_row_mark;
use crate::{generation, meta, StorageMode, StoreError, StoreResult, Table};

mod folding;

use folding::{count_prefix, drive_prefix, fold};

const MIGRATED_GENERATION: u64 = 1;

pub(super) fn rewrite(
    db: &Database,
    entities: &Keyspace,
    meta_keyspace: &Keyspace,
) -> StoreResult<Rewritten> {
    write_marker(db, meta_keyspace)?;
    let mut total = Rewritten::default();
    let mut marks: Vec<(Table, u64)> = Vec::new();
    for table in Table::ALL {
        match table.storage_mode() {
            StorageMode::ObservationLog => {
                marks.push((table, count_prefix(entities, &observation_prefix(table))?));
            }
            StorageMode::DerivedGeneration => {
                let collapsed = collapse(db, entities, table, Sink::Generation)?;
                total.rewritten = total
                    .rewritten
                    .checked_add(collapsed.rewritten)
                    .ok_or(StoreError::CounterOverflow)?;
                total.dropped = total
                    .dropped
                    .checked_add(collapsed.dropped)
                    .ok_or(StoreError::CounterOverflow)?;
                marks.push((table, collapsed.mark_rows));
            }
            StorageMode::DerivedMap => {
                let collapsed = collapse(db, entities, table, Sink::Map)?;
                total.dropped = total
                    .dropped
                    .checked_add(collapsed.dropped)
                    .ok_or(StoreError::CounterOverflow)?;
                marks.push((table, collapsed.mark_rows));
            }
        }
    }
    finish(db, meta_keyspace, &marks)?;
    Ok(total)
}

#[derive(Debug, Clone, Copy)]
enum Sink {
    Generation,
    Map,
}

#[derive(Debug, Default, Clone, Copy)]
struct Collapsed {
    mark_rows: u64,
    rewritten: u64,
    dropped: u64,
}

struct Pending {
    keys: Vec<Vec<u8>>,
    id: Vec<u8>,
    values: Vec<Vec<u8>>,
}

fn collapse(
    db: &Database,
    entities: &Keyspace,
    table: Table,
    sink: Sink,
) -> StoreResult<Collapsed> {
    let mut collapsed = Collapsed {
        mark_rows: match sink {
            Sink::Generation => count_prefix(
                entities,
                &derived_generation_prefix(table, MIGRATED_GENERATION),
            )?,
            Sink::Map => 0,
        },
        ..Collapsed::default()
    };
    let mut pending: Option<Pending> = None;
    drive_prefix(
        db,
        entities,
        &observation_prefix(table),
        |batch, key, value| {
            let (_, id, _) = view_observation_key(key).ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "table {} holds a malformed key {} during migration",
                    table.file(),
                    key_label(key)
                ),
            })?;
            match pending.as_mut() {
                Some(current) if current.id == id => {
                    current.keys.push(key.to_vec());
                    current.values.push(value.to_vec());
                    collapsed.dropped = collapsed
                        .dropped
                        .checked_add(1)
                        .ok_or(StoreError::CounterOverflow)?;
                    return Ok(());
                }
                Some(_) => {
                    flush_pending(batch, entities, table, sink, &mut pending, &mut collapsed)?
                }
                None => {}
            }
            pending = Some(Pending {
                keys: vec![key.to_vec()],
                id: id.to_vec(),
                values: vec![value.to_vec()],
            });
            Ok(())
        },
    )?;
    if let Some(current) = pending {
        finish_pending(db, entities, table, sink, &current, &mut collapsed)?;
    }
    Ok(collapsed)
}

fn finish_pending(
    db: &Database,
    entities: &Keyspace,
    table: Table,
    sink: Sink,
    current: &Pending,
    collapsed: &mut Collapsed,
) -> StoreResult<()> {
    let mut batch = db.batch();
    emit(&mut batch, entities, table, sink, current, collapsed)?;
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn flush_pending(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    sink: Sink,
    pending: &mut Option<Pending>,
    collapsed: &mut Collapsed,
) -> StoreResult<()> {
    let current = pending.take().ok_or_else(|| StoreError::Invariant {
        detail: format!("table {} lost a pending row during migration", table.file()),
    })?;
    emit(batch, entities, table, sink, &current, collapsed)
}

fn emit(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    sink: Sink,
    row: &Pending,
    collapsed: &mut Collapsed,
) -> StoreResult<()> {
    let folded = fold(table, &row.values)?;
    let id = std::str::from_utf8(&row.id).map_err(|_| StoreError::Invariant {
        detail: format!("table {} holds a non-utf8 id", table.file()),
    })?;
    match sink {
        Sink::Generation => {
            for key in &row.keys {
                batch.remove(entities, key.as_slice());
            }
            batch.insert(
                entities,
                derived_key(table, MIGRATED_GENERATION, id.as_bytes()),
                folded.as_slice(),
            );
            collapsed.rewritten = collapsed
                .rewritten
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
        }
        Sink::Map => {
            let settled = row.keys.len() == 1 && row.values.first() == Some(&folded);
            if !settled {
                for key in &row.keys {
                    batch.remove(entities, key.as_slice());
                }
                batch.insert(
                    entities,
                    observation_key(table, id, DERIVED_SEQUENCE),
                    folded.as_slice(),
                );
            }
        }
    }
    collapsed.mark_rows = collapsed
        .mark_rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

fn write_marker(db: &Database, meta_keyspace: &Keyspace) -> StoreResult<()> {
    let mut batch = db.batch();
    meta::put_text(&mut batch, meta_keyspace, MIGRATING_TO, "1");
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn finish(db: &Database, meta_keyspace: &Keyspace, marks: &[(Table, u64)]) -> StoreResult<()> {
    let mut batch = db.batch();
    for (table, rows) in marks {
        put_row_mark(&mut batch, meta_keyspace, *table, *rows);
    }
    generation::write_seed(
        &mut batch,
        meta_keyspace,
        0,
        MIGRATED_GENERATION,
        MIGRATED_GENERATION + 1,
        MIGRATED_GENERATION + 1,
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        STORE_VERSION,
        &STORE_SCHEMA_VERSION.to_string(),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        KEY_FORMAT,
        &KEY_FORMAT_VERSION.to_string(),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        CREATED_BY,
        env!("CARGO_PKG_VERSION"),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        CREATED_AT,
        &SystemClock.today_iso8601(),
    );
    batch.remove(meta_keyspace, MIGRATING_TO);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}
