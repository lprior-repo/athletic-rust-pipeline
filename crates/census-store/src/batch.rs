use std::collections::HashSet;

use fjall::{Keyspace, OwnedWriteBatch};
use serde::Serialize;

use super::keys::{
    derived_key, observation_id, observation_key, observation_prefix, DERIVED_SEQUENCE,
};
use super::{StorageMode, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

pub(super) fn refuse_over_bound(table: Table, rows: u64) -> StoreResult<()> {
    if rows > MAX_ROWS_PER_TABLE {
        return Err(StoreError::TooManyRows {
            table: table.file().to_string(),
            max: usize::try_from(MAX_ROWS_PER_TABLE).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

pub(super) fn refuse_observation_replacement(table: Table) -> StoreResult<()> {
    if table.storage_mode() == StorageMode::ObservationLog {
        return Err(StoreError::ObservationReplacement {
            table: table.file(),
        });
    }
    Ok(())
}

pub(super) fn refuse_derived_append(table: Table) -> StoreResult<()> {
    if table.storage_mode() == StorageMode::ObservationLog {
        return Ok(());
    }
    Err(StoreError::DerivedAppend {
        table: table.file(),
    })
}

const MAX_JOURNAL_LABEL_CHARS: usize = 64;

pub(super) fn refuse_over_journal(
    phase: &str,
    key: &str,
    what: &'static str,
    bytes: usize,
    max: usize,
) -> StoreResult<()> {
    if bytes > max {
        return Err(StoreError::JournalTooLarge {
            what,
            phase: label(phase),
            key: label(key),
            bytes,
            max,
        });
    }
    Ok(())
}

fn label(name: &str) -> String {
    name.chars().take(MAX_JOURNAL_LABEL_CHARS).collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Staged {
    pub(super) named: HashSet<String>,
    pub(super) added: u64,
}

pub(super) fn generation_row<T: Serialize>(
    table: Table,
    record: &T,
    generation: u64,
) -> StoreResult<(Vec<u8>, Vec<u8>)> {
    let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
        detail: "serializing derived state".to_string(),
        source,
    })?;
    let id = observation_id(&value)?;
    Ok((derived_key(table, generation, id.as_bytes()), value))
}

pub(super) fn stage_derived<T: Serialize>(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: &[T],
    generation: u64,
) -> StoreResult<Staged> {
    let mut encoded = Vec::with_capacity(records.len());
    for record in records {
        let (_, value) = generation_row(table, record, generation)?;
        encoded.push(value);
    }
    stage_derived_encoded(batch, entities, table, encoded, generation)
}

pub(super) fn stage_derived_encoded(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: Vec<Vec<u8>>,
    generation: u64,
) -> StoreResult<Staged> {
    match table.storage_mode() {
        StorageMode::DerivedGeneration => {
            stage_generation(batch, entities, table, records, generation)
        }
        StorageMode::DerivedMap => stage_map(batch, entities, table, records),
        StorageMode::ObservationLog => Err(StoreError::ObservationReplacement {
            table: table.file(),
        }),
    }
}

fn stage_generation(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: Vec<Vec<u8>>,
    generation: u64,
) -> StoreResult<Staged> {
    let mut staged = Staged {
        named: HashSet::with_capacity(records.len()),
        added: 0,
    };
    for value in records {
        let id = observation_id(&value)?;
        let key = derived_key(table, generation, id.as_bytes());
        let held = entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
            .is_some();
        let first_named = staged.named.insert(id);
        if first_named && !held {
            staged.added = staged.added.saturating_add(1);
        }
        batch.insert(entities, key, value);
    }
    Ok(staged)
}

fn stage_map(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: Vec<Vec<u8>>,
) -> StoreResult<Staged> {
    let mut staged = Staged {
        named: HashSet::with_capacity(records.len()),
        added: 0,
    };
    for value in records {
        let id = observation_id(&value)?;
        let key = observation_key(table, &id, DERIVED_SEQUENCE);
        let held = entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
            .is_some();
        let first_named = staged.named.insert(id);
        if first_named && !held {
            staged.added = staged.added.saturating_add(1);
        }
        batch.insert(entities, key, value);
    }
    drop_foreign_batch(entities, batch, table, &staged.named)?;
    Ok(staged)
}

fn drop_foreign_batch(
    entities: &Keyspace,
    batch: &mut OwnedWriteBatch,
    table: Table,
    named: &HashSet<String>,
) -> StoreResult<()> {
    if named.is_empty() {
        return Ok(());
    }
    let prefix = observation_prefix(table);
    for guard in entities.prefix(prefix.as_slice()) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, sequence) =
            super::keys::view_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} holds a malformed observation key", table.file()),
            })?;
        if sequence == DERIVED_SEQUENCE {
            continue;
        }
        let id = String::from_utf8_lossy(id).into_owned();
        if named.contains(&id) {
            batch.remove(entities, key);
        }
    }
    Ok(())
}
