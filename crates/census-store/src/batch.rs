use fjall::{Keyspace, OwnedWriteBatch};
use serde::Serialize;
use std::collections::HashSet;

use super::keys::{
    observation_id, observation_key, split_observation_key, table_prefix, DERIVED_SEQUENCE,
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

pub(super) struct Staged {
    pub(super) named: HashSet<String>,
    pub(super) added: u64,
}

impl Staged {
    pub(super) fn named_count(&self) -> StoreResult<u64> {
        u64::try_from(self.named.len()).map_err(|_| StoreError::CounterOverflow)
    }
}

pub(super) fn stage_derived<T: Serialize>(
    batch: &mut OwnedWriteBatch,
    entities: &Keyspace,
    table: Table,
    records: &[T],
) -> StoreResult<Staged> {
    let mut encoded = Vec::with_capacity(records.len());
    for record in records {
        let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
            detail: "serializing derived state".to_string(),
            source,
        })?;
        encoded.push(value);
    }
    stage_derived_encoded(batch, entities, table, encoded)
}

pub(super) fn stage_derived_encoded(
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
        let id = observation_id(&value)?.to_string();
        let key = observation_key(table, &id, DERIVED_SEQUENCE);
        let held = entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
            .is_some();
        let first_named = staged.named.insert(id.clone());
        if first_named && !held {
            staged.added = staged.added.saturating_add(1);
        }
        batch.insert(entities, key, value);
    }
    if table.storage_mode() != StorageMode::ObservationLog {
        drop_foreign_batch(entities, batch, table, &staged.named)?;
    }
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
    let prefix = table_prefix(table);
    for guard in entities.prefix(&prefix) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, sequence) =
            split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} holds a malformed observation key", table.file()),
            })?;
        let id_str = String::from_utf8_lossy(id).into_owned();
        if sequence != DERIVED_SEQUENCE && named.contains(&id_str) {
            batch.remove(entities, key);
        }
    }
    Ok(())
}

pub(super) fn drop_unnamed(
    entities: &Keyspace,
    batch: &mut OwnedWriteBatch,
    table: Table,
    named: &HashSet<String>,
) -> StoreResult<()> {
    let prefix = table_prefix(table);
    for guard in entities.prefix(&prefix) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (_, id, _) = split_observation_key(&key).ok_or_else(|| StoreError::Invariant {
            detail: format!("table {} holds a malformed observation key", table.file()),
        })?;
        let id_str = String::from_utf8_lossy(id);
        if !named.contains(id_str.as_ref()) {
            batch.remove(entities, key);
        }
    }
    Ok(())
}
