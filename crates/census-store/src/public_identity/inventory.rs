use super::{codec, PublicIdentityError};
use crate::{StoreError, StoreResult, Table};
use fjall::{Keyspace, Readable, Snapshot};

pub(super) fn validate(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    operations: usize,
    expected: usize,
) -> StoreResult<()> {
    Table::ALL.into_iter().try_for_each(|table| {
        let sequence = number(snapshot, meta, &format!("sequence:{}", table.file()))?;
        let rows = number(snapshot, meta, &format!("rows:{}", table.file()))?;
        crate::batch::refuse_over_bound(table, sequence)?;
        crate::batch::refuse_over_bound(table, rows)?;
        if table.storage_mode() == crate::StorageMode::ObservationLog {
            validate_observation_marks(snapshot, entities, table, sequence, rows)?;
        }
        Ok::<(), StoreError>(())
    })?;
    let evidence = number(snapshot, meta, crate::generation::ENTITIES)?;
    let current = number(snapshot, meta, crate::generation::DERIVED_CURRENT)?;
    let next = number(snapshot, meta, crate::generation::DERIVED_NEXT)?;
    let reclaim = number(snapshot, meta, crate::generation::DERIVED_RECLAIM_FROM)?;
    let expected_u64 = u64::try_from(expected).map_err(|_| StoreError::CounterOverflow)?;
    let operations = u64::try_from(operations).map_err(|_| StoreError::CounterOverflow)?;
    if evidence < operations || next <= current || reclaim > next {
        return Err(PublicIdentityError::Corrupt("generation marks").into());
    }
    let table = Table::AthleteIdentityDecisions;
    let rows = number(snapshot, meta, &format!("rows:{}", table.file()))?;
    let count = snapshot
        .prefix(entities, crate::keys::observation_prefix(table))
        .take(expected.saturating_add(1))
        .try_fold(0usize, |count, guard| {
            guard.key().map_err(|source| StoreError::Read { source })?;
            count.checked_add(1).ok_or(StoreError::CounterOverflow)
        })?;
    if rows != expected_u64 || count != expected {
        return Err(PublicIdentityError::Corrupt("accepted decision cardinality").into());
    }
    Ok(())
}

pub(super) fn number(snapshot: &Snapshot, meta: &Keyspace, key: &str) -> StoreResult<u64> {
    let raw = codec::load(snapshot, meta, key, 32)?
        .ok_or(PublicIdentityError::Corrupt("missing retained counter"))?;
    let text =
        std::str::from_utf8(&raw).map_err(|_| PublicIdentityError::Corrupt("counter encoding"))?;
    crate::meta::parse_u64(key, text)
}

fn validate_observation_marks(
    snapshot: &Snapshot,
    entities: &Keyspace,
    table: Table,
    next_sequence: u64,
    expected_rows: u64,
) -> StoreResult<()> {
    let limit = expected_rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let limit = usize::try_from(limit).map_err(|_| StoreError::CounterOverflow)?;
    let rows = snapshot
        .prefix(entities, crate::keys::observation_prefix(table))
        .take(limit)
        .try_fold(0_u64, |count, guard| {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            let (_, id, sequence) = crate::keys::view_observation_key(&key)
                .ok_or(PublicIdentityError::Corrupt("observation key"))?;
            if id.is_empty() || id.len() > crate::MAX_ID_BYTES || sequence >= next_sequence {
                return Err(PublicIdentityError::Corrupt("observation sequence mark").into());
            }
            count.checked_add(1).ok_or(StoreError::CounterOverflow)
        })?;
    if rows != expected_rows {
        return Err(PublicIdentityError::Corrupt("observation row mark").into());
    }
    Ok(())
}
