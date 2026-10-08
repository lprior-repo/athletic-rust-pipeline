use std::ops::Bound;

use fjall::{Database, Keyspace, OwnedWriteBatch, PersistMode};

use super::{exact, owned, Rewritten};
use crate::{meta, StoreError, StoreResult, Table};

const PERFORMANCE_CURSOR: &str = "schema:exacttime:performances_cursor";
const JOURNAL_CURSOR: &str = "schema:exacttime:journal_cursor";
const REWRITTEN: &str = "schema:exacttime:rewritten";
const ARCHIVED: &str = "schema:exacttime:archived_owned_rows";
const MAX_ROW_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct Context<'a> {
    pub(super) db: &'a Database,
    pub(super) entities: &'a Keyspace,
    pub(super) journal: &'a Keyspace,
    pub(super) meta: &'a Keyspace,
}

pub(super) fn migrate(context: &Context<'_>) -> StoreResult<Rewritten> {
    let prefix = crate::keys::observation_prefix(Table::Performances);
    context.walk(
        context.entities,
        &prefix,
        PERFORMANCE_CURSOR,
        |batch, key, value| {
            if let Some(converted) = exact::convert_performance(key, value)? {
                check_size(&converted)?;
                batch.insert(context.entities, key, converted);
                context.increment(batch, REWRITTEN)?;
            }
            Ok(())
        },
    )?;
    context.walk(context.journal, b"", JOURNAL_CURSOR, |batch, key, value| {
        context.rekey_owned(batch, key, value)
    })?;
    Ok(Rewritten {
        rewritten: meta::get_u64(context.meta, REWRITTEN)?.map_or(0, core::convert::identity),
        dropped: 0,
    })
}

impl Context<'_> {
    fn walk<F>(
        &self,
        keyspace: &Keyspace,
        prefix: &[u8],
        cursor_name: &str,
        mut step: F,
    ) -> StoreResult<()>
    where
        F: FnMut(&mut OwnedWriteBatch, &[u8], &[u8]) -> StoreResult<()>,
    {
        let cursor = self
            .meta
            .get(cursor_name)
            .map_err(|source| StoreError::Read { source })?;
        let start = cursor.as_deref().map_or(prefix, core::convert::identity);
        if !start.starts_with(prefix) {
            return Err(exact::invalid(
                "resume cursor is outside its migration table",
            ));
        }
        keyspace
            .range::<&[u8], _>((Bound::Excluded(start), Bound::Unbounded))
            .map(|guard| {
                guard
                    .into_inner()
                    .map_err(|source| StoreError::Read { source })
            })
            .map_while(|row| match row {
                Ok((key, value)) if key.starts_with(prefix) => Some(Ok((key, value))),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .try_for_each(|row| {
                let (key, value) = row?;
                check_size(&value)?;
                let mut batch = self.db.batch();
                step(&mut batch, &key, &value)?;
                batch.insert(self.meta, cursor_name, key.as_ref());
                commit(batch)
            })
    }

    fn increment(&self, batch: &mut OwnedWriteBatch, name: &str) -> StoreResult<()> {
        let current = meta::get_u64(self.meta, name)?.map_or(0, core::convert::identity);
        let next = current.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        meta::put_text(batch, self.meta, name, &next.to_string());
        Ok(())
    }

    fn rekey_owned(
        &self,
        batch: &mut OwnedWriteBatch,
        key: &[u8],
        bytes: &[u8],
    ) -> StoreResult<()> {
        let Some(converted) = owned::convert(key, bytes)? else {
            return Ok(());
        };
        check_size(&converted.value)?;
        self.refuse_collision(&converted.archive, bytes)?;
        batch.insert(self.journal, converted.archive, bytes);
        self.stage_current(batch, &converted.key, &converted.value)?;
        batch.remove(self.journal, key);
        self.increment(batch, ARCHIVED)?;
        self.increment(batch, REWRITTEN)
    }

    fn stage_current(
        &self,
        batch: &mut OwnedWriteBatch,
        key: &[u8],
        value: &[u8],
    ) -> StoreResult<()> {
        let Some(held) = self
            .journal
            .get(key)
            .map_err(|source| StoreError::Read { source })?
        else {
            batch.insert(self.journal, key, value);
            return Ok(());
        };
        check_size(&held)?;
        let prior = exact::decode(&held)?;
        let current = exact::decode(value)?;
        if prior.get("key") != current.get("key") || prior.get("payload") != current.get("payload")
        {
            return Err(exact::invalid(
                "owned row rekey collides with different payload or locator",
            ));
        }
        Ok(())
    }

    fn refuse_collision(&self, key: &[u8], value: &[u8]) -> StoreResult<()> {
        let held = self
            .journal
            .get(key)
            .map_err(|source| StoreError::Read { source })?;
        if held.as_deref().is_some_and(|held| held != value) {
            return Err(exact::invalid(
                "owned row rekey collides with different immutable bytes",
            ));
        }
        Ok(())
    }
}

fn check_size(bytes: &[u8]) -> StoreResult<()> {
    if bytes.len() > MAX_ROW_BYTES {
        return Err(exact::invalid("row exceeds the 4 MiB migration row bound"));
    }
    Ok(())
}

pub(super) fn commit(batch: OwnedWriteBatch) -> StoreResult<()> {
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

pub(super) fn clear_cursors(batch: &mut OwnedWriteBatch, meta: &Keyspace) {
    [PERFORMANCE_CURSOR, JOURNAL_CURSOR, REWRITTEN]
        .into_iter()
        .for_each(|name| batch.remove(meta, name));
}
