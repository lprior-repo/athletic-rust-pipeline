use std::ops::{Bound, ControlFlow};

use fjall::Readable;

use crate::keys::{derived_key, view_observation_key};
use crate::{Entity, StoreError, StoreResult, Table, MAX_ID_BYTES, MAX_ROWS_PER_TABLE};

use super::merge;
use super::StoreSnapshot;

impl StoreSnapshot<'_> {
    pub fn get_merged<T: Entity>(&self, table: Table, id: &str) -> StoreResult<Option<T>> {
        if id.is_empty() || id.len() > MAX_ID_BYTES {
            return Ok(None);
        }
        if table.generation_partitioned() {
            let key = derived_key(table, self.derived_generation, id.as_bytes());
            return self
                .snapshot
                .get(self.entities, &key)
                .map_err(|source| StoreError::Read { source })?
                .map(|raw| {
                    let mut row: T = merge::decode(table, &key, &raw)?;
                    row.publish();
                    Ok(row)
                })
                .transpose();
        }
        let mut prefix = self.layout().prefix(table);
        prefix.extend_from_slice(id.as_bytes());
        prefix.push(0);
        let mut merged: Option<T> = None;
        let max = usize::try_from(MAX_ROWS_PER_TABLE).map_err(|_| StoreError::CounterOverflow)?;
        for (index, guard) in self.snapshot.prefix(self.entities, &prefix).enumerate() {
            merge::check_limit(index, max, table)?;
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let (_, stored_id, _) =
                view_observation_key(&key).ok_or_else(|| invalid("malformed keyed observation"))?;
            if stored_id != id.as_bytes() {
                continue;
            }
            let row = merge::decode(table, &key, &raw)?;
            match merged.as_mut() {
                Some(current) => current.merge(row),
                None => merged = Some(row),
            }
        }
        if let Some(row) = merged.as_mut() {
            row.publish();
        }
        Ok(merged)
    }

    pub fn for_each_merged_after<T: Entity>(
        &self,
        table: Table,
        after: Option<&str>,
        mut visit: impl FnMut(T) -> StoreResult<ControlFlow<()>>,
    ) -> StoreResult<u64> {
        if !table.generation_partitioned() {
            return Err(invalid(
                "cursor reads require a generation-partitioned table",
            ));
        }
        if let Some(id) = after {
            check_cursor(id)?;
        }
        let prefix = self.layout().prefix(table);
        let start = match after {
            Some(id) => Bound::Excluded(derived_key(table, self.derived_generation, id.as_bytes())),
            None => Bound::Included(prefix.clone()),
        };
        let mut end = prefix;
        let separator = end
            .last_mut()
            .ok_or_else(|| invalid("empty table prefix"))?;
        *separator = separator
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let max = usize::try_from(MAX_ROWS_PER_TABLE).map_err(|_| StoreError::CounterOverflow)?;
        let mut visited = 0_u64;
        for (index, guard) in self
            .snapshot
            .range(self.entities, (start, Bound::Excluded(end)))
            .enumerate()
        {
            merge::check_limit(index, max, table)?;
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let mut row: T = merge::decode(table, &key, &raw)?;
            row.publish();
            visited = visited.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            if visit(row)?.is_break() {
                break;
            }
        }
        Ok(visited)
    }
}

fn check_cursor(id: &str) -> StoreResult<()> {
    let max = MAX_ID_BYTES
        .checked_add(128)
        .ok_or(StoreError::CounterOverflow)?;
    if id.len() > max {
        return Err(invalid(
            "cursor exceeds the bounded derived identifier ceiling",
        ));
    }
    Ok(())
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: detail.to_string(),
    }
}

#[cfg(test)]
#[path = "keyed_tests.rs"]
mod tests;
