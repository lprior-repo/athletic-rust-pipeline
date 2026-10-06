use std::ops::Bound;

use fjall::PersistMode;

use super::super::keys::derived_generation_prefix;
use super::super::{generation, meta, Store, StoreError, StoreResult, Table};

#[cfg(test)]
mod tests;

const CHUNK: usize = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reclaimed {
    pub rows: u64,
    pub complete: bool,
}

pub(super) fn reclaim(store: &Store, budget: u64) -> StoreResult<Reclaimed> {
    let _staging = store.lock_staging();
    let _appends = store.lock_appends();
    let current = store.derived_generation();
    let next = meta::get_u64(&store.meta, generation::DERIVED_NEXT)?.ok_or_else(|| {
        StoreError::SchemaUnknown {
            detail: format!(
                "{} is missing from a versioned store",
                generation::DERIVED_NEXT
            ),
        }
    })?;
    let stored =
        meta::get_u64(&store.meta, generation::DERIVED_RECLAIM_FROM)?.ok_or_else(|| {
            StoreError::SchemaUnknown {
                detail: format!(
                    "{} is missing from a versioned store",
                    generation::DERIVED_RECLAIM_FROM
                ),
            }
        })?;
    let mut remaining = budget;
    let mut rows = 0_u64;
    let mut cursor = stored.min(current);
    let mut resume = cursor;
    while cursor < next && remaining > 0 {
        if cursor == current {
            cursor = cursor.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            continue;
        }
        if !reclaim_generation(store, cursor, &mut remaining, &mut rows)? {
            break;
        }
        cursor = cursor.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        resume = cursor;
    }
    let from = resume.min(current);
    let mut batch = store.db.batch();
    meta::put_text(
        &mut batch,
        &store.meta,
        generation::DERIVED_RECLAIM_FROM,
        &from.to_string(),
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(Reclaimed {
        rows,
        complete: cursor >= next,
    })
}

fn reclaim_generation(
    store: &Store,
    generation: u64,
    remaining: &mut u64,
    rows: &mut u64,
) -> StoreResult<bool> {
    for table in Table::ALL {
        if !table.generation_partitioned() {
            continue;
        }
        let prefix = derived_generation_prefix(table, generation);
        if !delete_prefix(store, &prefix, remaining, rows)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn delete_prefix(
    store: &Store,
    prefix: &[u8],
    budget: &mut u64,
    rows: &mut u64,
) -> StoreResult<bool> {
    let mut cursor: Option<Vec<u8>> = None;
    loop {
        if *budget == 0 {
            return Ok(false);
        }
        let start = match &cursor {
            Some(last) => last.clone(),
            None => prefix.to_vec(),
        };
        let mut keys = collect_chunk(store, prefix, &start)?;
        let collected = keys.len();
        if collected == 0 {
            return Ok(true);
        }
        let permitted = permitted_rows(*budget, collected)?;
        if permitted < collected {
            keys.truncate(permitted);
            remove_chunk(store, &keys)?;
            charge(budget, rows, keys.len())?;
            return Ok(false);
        }
        remove_chunk(store, &keys)?;
        charge(budget, rows, collected)?;
        cursor = keys.pop();
        if collected < CHUNK {
            return Ok(true);
        }
    }
}

fn collect_chunk(store: &Store, prefix: &[u8], start: &[u8]) -> StoreResult<Vec<Vec<u8>>> {
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for guard in store
        .entities
        .range((Bound::Excluded(start.to_vec()), Bound::Unbounded))
    {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if !key.starts_with(prefix) {
            break;
        }
        keys.push(key.to_vec());
        if keys.len() >= CHUNK {
            break;
        }
    }
    Ok(keys)
}

fn permitted_rows(budget: u64, collected: usize) -> StoreResult<usize> {
    let collected = u64::try_from(collected).map_err(|_| StoreError::CounterOverflow)?;
    usize::try_from(budget.min(collected)).map_err(|_| StoreError::CounterOverflow)
}

fn remove_chunk(store: &Store, keys: &[Vec<u8>]) -> StoreResult<()> {
    let mut batch = store.db.batch();
    for key in keys {
        batch.remove(&store.entities, key.as_slice());
    }
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn charge(budget: &mut u64, rows: &mut u64, deleted: usize) -> StoreResult<()> {
    let deleted = u64::try_from(deleted).map_err(|_| StoreError::CounterOverflow)?;
    *budget = budget
        .checked_sub(deleted)
        .ok_or(StoreError::CounterOverflow)?;
    *rows = rows
        .checked_add(deleted)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(())
}
