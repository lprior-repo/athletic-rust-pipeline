use crate::keys::{key_label, view_derived_key, view_observation_key};
use crate::{Entity, StoreError, StoreResult, Table};

pub(super) fn check_limit(index: usize, max: usize, table: Table) -> StoreResult<()> {
    if index >= max {
        return Err(StoreError::TooManyRows {
            table: table.file().to_owned(),
            max,
        });
    }
    Ok(())
}

pub(super) fn decode<T: Entity>(table: Table, key: &[u8], raw: &[u8]) -> StoreResult<T> {
    let id = if table.generation_partitioned() {
        let (_, id) = view_derived_key(table, key).ok_or_else(|| StoreError::Invariant {
            detail: format!("malformed derived key {}", key_label(key)),
        })?;
        id
    } else {
        let (key_table, key_id, _) =
            view_observation_key(key).ok_or_else(|| StoreError::Invariant {
                detail: format!("malformed observation key {}", key_label(key)),
            })?;
        if key_table != table.file().as_bytes() {
            return Err(StoreError::Invariant {
                detail: format!(
                    "observation key {} does not identify a row of {}",
                    key_label(key),
                    table.file()
                ),
            });
        }
        key_id
    };
    if id.is_empty() {
        return Err(StoreError::Invariant {
            detail: format!("key {} identifies no row id", key_label(key)),
        });
    }
    let row: T = serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: key_label(key),
        source,
    })?;
    if row.entity_id().as_bytes() != id {
        return Err(StoreError::Invariant {
            detail: format!("key {} disagrees with its payload identity", key_label(key)),
        });
    }
    Ok(row)
}

pub(super) fn accept<T: Entity>(
    current: &mut Option<T>,
    row: T,
    visit: &mut impl FnMut(T) -> StoreResult<()>,
    published: u64,
) -> StoreResult<u64> {
    if let Some(entity) = current
        .as_mut()
        .filter(|entity| entity.entity_id() == row.entity_id())
    {
        entity.merge(row);
        return Ok(published);
    }
    match current.replace(row) {
        Some(previous) => publish(previous, visit, published),
        None => Ok(published),
    }
}

pub(super) fn publish<T: Entity>(
    mut row: T,
    visit: &mut impl FnMut(T) -> StoreResult<()>,
    published: u64,
) -> StoreResult<u64> {
    let next = published
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    row.publish();
    visit(row)?;
    Ok(next)
}
