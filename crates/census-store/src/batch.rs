use std::collections::HashSet;

use fjall::{Keyspace, OwnedWriteBatch};
use serde::Serialize;

use super::keys::{
    derived_key, observation_id, observation_key, observation_prefix, DERIVED_SEQUENCE,
};
use super::{StorageMode, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

#[cfg(test)]
mod tests;

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
        #[cfg(test)]
        tests::point_read()?;
        let held = entities
            .get(&key)
            .map_err(|source| StoreError::Read { source })?
            .is_some();
        let first_named = staged.named.insert(id);
        if first_named && !held {
            staged.added = staged
                .added
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
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
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(named.len())
        .map_err(|_| StoreError::Refused {
            detail: "allocating derived map cleanup prefixes".into(),
        })?;
    ordered.extend(named.iter().map(String::as_str));
    ordered.sort_unstable();
    let mut covered: Option<&str> = None;
    let mut visits = 0;
    let mut prefix = cleanup_prefix(table)?;
    let table_bytes = prefix.len();
    for id in ordered {
        if covered.is_some_and(|prior| {
            id.strip_prefix(prior)
                .is_some_and(|suffix| suffix.starts_with('\0'))
        }) {
            continue;
        }
        covered = Some(id);
        prefix.truncate(table_bytes);
        prefix.extend_from_slice(id.as_bytes());
        prefix.push(0);
        for guard in entities.prefix(prefix.as_slice()) {
            charge_cleanup(&mut visits)?;
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            #[cfg(test)]
            tests::key_visit()?;
            let (_, id, sequence) =
                super::keys::view_observation_key(&key).ok_or_else(|| StoreError::Invariant {
                    detail: format!("table {} holds a malformed observation key", table.file()),
                })?;
            if sequence == DERIVED_SEQUENCE {
                continue;
            }
            let id = std::str::from_utf8(id).map_err(|_| StoreError::Invariant {
                detail: format!("table {} holds a non-utf8 map id", table.file()),
            })?;
            #[cfg(test)]
            tests::comparison()?;
            if named.contains(id) {
                batch.remove(entities, key);
            }
        }
    }
    Ok(())
}

fn cleanup_prefix(table: Table) -> StoreResult<Vec<u8>> {
    let mut prefix = observation_prefix(table);
    let extra = crate::MAX_ID_BYTES
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    prefix
        .try_reserve_exact(extra)
        .map_err(|_| StoreError::Refused {
            detail: "allocating derived map cleanup key buffer".into(),
        })?;
    Ok(prefix)
}

fn charge_cleanup(visits: &mut u64) -> StoreResult<()> {
    let next = visits.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    if next > MAX_ROWS_PER_TABLE {
        return Err(StoreError::BudgetExceeded {
            what: "derived map cleanup key visits",
            needed: next,
            available: MAX_ROWS_PER_TABLE,
        });
    }
    *visits = next;
    Ok(())
}
