use super::{Page, Replacement, StoreBatch};
use crate::receipt::{self, Decision};
use crate::{StoreError, StoreResult};
use fjall::OwnedWriteBatch;
use std::collections::HashSet;

pub struct ConditionalBatch<'store, 'identity> {
    batch: StoreBatch<'store>,
    operation: &'identity str,
    digest: &'identity str,
}

impl<'store> StoreBatch<'store> {
    pub fn conditional<'identity>(
        self,
        operation: &'identity str,
        digest: &'identity str,
    ) -> StoreResult<ConditionalBatch<'store, 'identity>> {
        receipt::refuse_over_operation(operation, digest)?;
        Ok(ConditionalBatch {
            batch: self,
            operation,
            digest,
        })
    }
}

pub(super) fn admit(
    base: &StoreBatch<'_>,
    effects: &[ConditionalBatch<'_, '_>],
    operation: &str,
) -> StoreResult<()> {
    if effects.len() > 100_000 {
        return Err(refused("conditional effect count exceeds 100000"));
    }
    let mut seen = HashSet::new();
    seen.try_reserve(effects.len())
        .map_err(|_| refused("allocating conditional operation membership"))?;
    let mut bytes = encoded_bytes(base)?;
    if bytes > 32 * 1024 * 1024 {
        return Err(refused("conditional staged bytes exceed 33554432"));
    }
    effects.iter().try_for_each(|effect| {
        if !std::ptr::eq(base.store, effect.batch.store) {
            return Err(refused("conditional batch belongs to another store"));
        }
        if effect.operation == operation {
            return Err(refused("outer operation is also a conditional effect"));
        }
        if !seen.insert(effect.operation) {
            return Err(refused(
                "conditional operation occurs twice in one application",
            ));
        }
        bytes = bytes
            .checked_add(encoded_bytes(&effect.batch)?)
            .ok_or(StoreError::CounterOverflow)?;
        if bytes > 32 * 1024 * 1024 {
            return Err(refused("conditional staged bytes exceed 33554432"));
        }
        Ok(())
    })
}

pub(super) fn merge<'store>(
    mut base: StoreBatch<'store>,
    effects: Vec<ConditionalBatch<'store, '_>>,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<StoreBatch<'store>> {
    effects.into_iter().try_for_each(|effect| {
        if matches!(
            receipt::decide(base.store, effect.operation, effect.digest)?,
            Decision::Repeat(_)
        ) {
            return Ok(());
        }
        let count = super::commit::count_rows(&effect.batch.pages)?;
        super::commit::first_application(
            base.store,
            (effect.operation, effect.digest),
            count,
            batch,
        )?;
        merge_pages(&mut base.pages, effect.batch.pages)?;
        base.journal
            .try_reserve(effect.batch.journal.len())
            .map_err(|_| refused("allocating conditional journals"))?;
        base.journal.extend(effect.batch.journal);
        merge_replacements(&mut base.replacements, effect.batch.replacements)
    })?;
    Ok(base)
}

fn merge_pages(pages: &mut Vec<Page>, incoming: Vec<Page>) -> StoreResult<()> {
    incoming.into_iter().try_for_each(|page| {
        if let Some(standing) = pages
            .iter_mut()
            .find(|standing| standing.table == page.table)
        {
            standing
                .records
                .try_reserve(page.records.len())
                .map_err(|_| refused("allocating conditional rows"))?;
            standing.records.extend(page.records);
        } else {
            pages
                .try_reserve(1)
                .map_err(|_| refused("allocating conditional tables"))?;
            pages.push(page);
        }
        Ok(())
    })
}

fn merge_replacements(
    standing: &mut Vec<Replacement>,
    incoming: Vec<Replacement>,
) -> StoreResult<()> {
    incoming.into_iter().try_for_each(|replacement| {
        if standing.iter().any(|held| held.table == replacement.table) {
            return Err(refused(
                "derived table is replaced twice in conditional application",
            ));
        }
        standing
            .try_reserve(1)
            .map_err(|_| refused("allocating conditional replacements"))?;
        standing.push(replacement);
        Ok(())
    })
}

fn encoded_bytes(batch: &StoreBatch<'_>) -> StoreResult<usize> {
    let pages = batch
        .pages
        .iter()
        .flat_map(|page| page.records.iter())
        .map(|(key, value)| (key.len(), value.len()));
    let journal = batch
        .journal
        .iter()
        .map(|(key, value)| (key.len(), value.len()));
    let replacements = batch
        .replacements
        .iter()
        .flat_map(|page| page.records.iter())
        .map(|value| (0, value.len()));
    pages
        .chain(journal)
        .chain(replacements)
        .try_fold(0usize, |total, (key, value)| {
            total
                .checked_add(key)
                .and_then(|total| total.checked_add(value))
                .ok_or(StoreError::CounterOverflow)
        })
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}
