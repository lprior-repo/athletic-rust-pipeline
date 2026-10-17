use super::{Page, Replacement, StoreBatch};
use crate::receipt::{self, Decision};
use crate::{StoreError, StoreResult};
use fjall::OwnedWriteBatch;
use std::collections::HashSet;

pub(super) const MAX_STAGED_BYTES: usize = 32 * 1024 * 1024;

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
    admit_offered_observations(base, effects)?;
    let mut seen = HashSet::new();
    seen.try_reserve(effects.len())
        .map_err(|_| refused("allocating conditional operation membership"))?;
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
        Ok(())
    })
}

fn admit_offered_observations(
    base: &StoreBatch<'_>,
    effects: &[ConditionalBatch<'_, '_>],
) -> StoreResult<()> {
    std::iter::once(base)
        .chain(effects.iter().map(|effect| &effect.batch))
        .try_fold((0usize, 0usize), |(rows, bytes), batch| {
            let (incoming_rows, incoming_bytes) = batch.pending_counts()?;
            let rows = rows
                .checked_add(incoming_rows)
                .ok_or(StoreError::CounterOverflow)?;
            let bytes = bytes
                .checked_add(incoming_bytes)
                .ok_or(StoreError::CounterOverflow)?;
            super::source_access::admit_counts(rows, bytes)?;
            Ok::<_, StoreError>((rows, bytes))
        })?;
    Ok(())
}

pub(super) fn admit_bytes(
    base: &StoreBatch<'_>,
    effects: &[ConditionalBatch<'_, '_>],
) -> StoreResult<()> {
    let mut bytes = encoded_bytes(base)?;
    if bytes > MAX_STAGED_BYTES {
        return Err(refused("conditional staged bytes exceed 33554432"));
    }
    effects.iter().try_for_each(|effect| {
        bytes = bytes
            .checked_add(encoded_bytes(&effect.batch)?)
            .ok_or(StoreError::CounterOverflow)?;
        if bytes > MAX_STAGED_BYTES {
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
        merge_replacements(&mut base.replacements, effect.batch.replacements)?;
        base.source_access
            .try_reserve(effect.batch.source_access.len())
            .map_err(|_| refused("allocating conditional source access rows"))?;
        base.source_access.extend(effect.batch.source_access);
        Ok::<_, StoreError>(())
    })?;
    base.admit_observations()?;
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
    let source_access = batch
        .source_access
        .iter()
        .map(|(key, value)| (key.len(), value.len()));
    pages
        .chain(journal)
        .chain(replacements)
        .chain(source_access)
        .try_fold(0usize, |total, (key, value)| {
            total
                .checked_add(key)
                .and_then(|total| total.checked_add(value))
                .ok_or(StoreError::CounterOverflow)
        })
}

pub(super) fn source_access_capacity(batch: &StoreBatch<'_>) -> StoreResult<Option<usize>> {
    if batch.source_access.is_empty() {
        return Ok(None);
    }
    let Some(limit) = batch.conditional_byte_limit else {
        return Ok(None);
    };
    let pending = batch
        .source_access
        .iter()
        .try_fold(0usize, |total, (key, value)| {
            total
                .checked_add(key.len())
                .and_then(|total| total.checked_add(value.len()))
                .ok_or(StoreError::CounterOverflow)
        })?;
    let other = encoded_bytes(batch)?
        .checked_sub(pending)
        .ok_or(StoreError::CounterOverflow)?;
    let available = limit
        .checked_sub(other)
        .ok_or_else(|| refused("conditional staged bytes exceed 33554432"))?;
    Ok(Some(available))
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}
