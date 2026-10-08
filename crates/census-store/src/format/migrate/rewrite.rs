use fjall::{Keyspace, OwnedWriteBatch};

use super::super::{CREATED_AT, CREATED_BY, KEY_FORMAT, KEY_FORMAT_VERSION, STORE_VERSION};
use super::time::{commit, Context};
use super::Rewritten;
use crate::clock::{Clock, SystemClock};
use crate::keys::{
    derived_generation_prefix, derived_key, observation_key, observation_prefix,
    view_observation_key,
};
use crate::{generation, meta, StorageMode, StoreError, StoreResult, Table};

mod folding;

const REWRITTEN: &str = "schema:legacy:rewritten";
const DROPPED: &str = "schema:legacy:dropped";
const MAX_ROW_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn rewrite(context: &Context<'_>) -> StoreResult<Rewritten> {
    Table::ALL
        .into_iter()
        .try_for_each(|table| collapse(context, table))?;
    finish(context)?;
    Ok(Rewritten {
        rewritten: count(context.meta, REWRITTEN)?,
        dropped: count(context.meta, DROPPED)?,
    })
}

fn collapse(context: &Context<'_>, table: Table) -> StoreResult<()> {
    if table.storage_mode() == StorageMode::ObservationLog {
        return Ok(());
    }
    context
        .entities
        .prefix(observation_prefix(table))
        .try_for_each(|guard| {
            let (key, value) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            collapse_row(context, table, &key, &value)
        })
}

fn collapse_row(context: &Context<'_>, table: Table, key: &[u8], value: &[u8]) -> StoreResult<()> {
    bounded(value)?;
    let target = target_key(table, key)?;
    let prior = context
        .entities
        .get(&target)
        .map_err(|source| StoreError::Read { source })?;
    if target == key {
        bounded(&folding::fold(table, None, value)?)?;
        return Ok(());
    }
    if let Some(prior) = &prior {
        bounded(prior)?;
    }
    let folded = folding::fold(table, prior.as_deref(), value)?;
    bounded(&folded)?;
    let mut batch = context.db.batch();
    batch.remove(context.entities, key);
    batch.insert(context.entities, target, folded);
    if prior.is_some() {
        increment(context, &mut batch, DROPPED)?;
    } else if table.generation_partitioned() {
        increment(context, &mut batch, REWRITTEN)?;
    }
    commit(batch)
}

fn target_key(table: Table, key: &[u8]) -> StoreResult<Vec<u8>> {
    let (_, id, _) = view_observation_key(key).ok_or_else(|| StoreError::Invariant {
        detail: format!("malformed {} key during legacy migration", table.file()),
    })?;
    match table.storage_mode() {
        StorageMode::DerivedGeneration => Ok(derived_key(table, 1, id)),
        StorageMode::DerivedMap => {
            let id = std::str::from_utf8(id).map_err(|_| StoreError::Invariant {
                detail: "legacy map key has a non-utf8 id".into(),
            })?;
            Ok(observation_key(table, id, 0))
        }
        StorageMode::ObservationLog => Err(StoreError::Invariant {
            detail: "legacy collapse cannot rewrite an observation log".into(),
        }),
    }
}

fn finish(context: &Context<'_>) -> StoreResult<()> {
    let mut batch = context.db.batch();
    Table::ALL.into_iter().try_for_each(|table| {
        let prefix = if table.generation_partitioned() {
            derived_generation_prefix(table, 1)
        } else {
            observation_prefix(table)
        };
        let rows = context
            .entities
            .prefix(prefix)
            .try_fold(0_u64, |count, guard| {
                guard.key().map_err(|source| StoreError::Read { source })?;
                count.checked_add(1).ok_or(StoreError::CounterOverflow)
            })?;
        crate::rows::put_row_mark(&mut batch, context.meta, table, rows);
        Ok::<(), StoreError>(())
    })?;
    generation::write_seed(&mut batch, context.meta, 0, 1, 2, 2);
    meta::put_text(&mut batch, context.meta, STORE_VERSION, "1");
    meta::put_text(
        &mut batch,
        context.meta,
        KEY_FORMAT,
        &KEY_FORMAT_VERSION.to_string(),
    );
    preserve_creation(context, &mut batch)?;
    commit(batch)
}

fn preserve_creation(context: &Context<'_>, batch: &mut OwnedWriteBatch) -> StoreResult<()> {
    if meta::get_text(context.meta, CREATED_BY)?.is_none() {
        meta::put_text(batch, context.meta, CREATED_BY, env!("CARGO_PKG_VERSION"));
    }
    if meta::get_text(context.meta, CREATED_AT)?.is_none() {
        meta::put_text(
            batch,
            context.meta,
            CREATED_AT,
            &SystemClock.today_iso8601(),
        );
    }
    Ok(())
}

fn count(meta: &Keyspace, key: &str) -> StoreResult<u64> {
    Ok(meta::get_u64(meta, key)?.map_or(0, core::convert::identity))
}

fn increment(context: &Context<'_>, batch: &mut OwnedWriteBatch, key: &str) -> StoreResult<()> {
    let next = count(context.meta, key)?
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    meta::put_text(batch, context.meta, key, &next.to_string());
    Ok(())
}

fn bounded(bytes: &[u8]) -> StoreResult<()> {
    if bytes.len() > MAX_ROW_BYTES {
        return Err(StoreError::Refused {
            detail: "legacy migration row exceeds 4 MiB".into(),
        });
    }
    Ok(())
}

pub(super) fn statistics(meta: &Keyspace) -> StoreResult<Rewritten> {
    Ok(Rewritten {
        rewritten: count(meta, REWRITTEN)?,
        dropped: count(meta, DROPPED)?,
    })
}

pub(super) fn clear_statistics(batch: &mut OwnedWriteBatch, meta: &Keyspace) {
    [REWRITTEN, DROPPED]
        .into_iter()
        .for_each(|key| batch.remove(meta, key));
}
