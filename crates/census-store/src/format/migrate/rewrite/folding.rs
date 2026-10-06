use std::ops::Bound;

use fjall::{Database, Keyspace, OwnedWriteBatch, PersistMode};

use crate::{Entity, StoreError, StoreResult, Table};
use census_domain::model::{
    AppliedAthleteIdentity, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition, SourceObjectIdentity,
};

const MIGRATION_BATCH_ROWS: usize = 8192;

pub(super) fn fold(table: Table, values: &[Vec<u8>]) -> StoreResult<Vec<u8>> {
    match table {
        Table::SourceIdentities => fold_rows::<SourceObjectIdentity>(table, values),
        Table::Conflicts => fold_rows::<RetainedConflict>(table, values),
        Table::Coverage => fold_rows::<CoverageRow>(table, values),
        Table::ReviewCases => fold_rows::<ReviewCase>(table, values),
        Table::Snapshots => fold_rows::<CollectionSnapshot>(table, values),
        Table::SourceAccess => fold_rows::<SourceAccessCondition>(table, values),
        Table::IdentityVerdicts => fold_rows::<ReviewVerdictRecord>(table, values),
        Table::AthleteIdentityDecisions => fold_rows::<AppliedAthleteIdentity>(table, values),
        _ => Err(StoreError::Invariant {
            detail: format!("table {} is not stored as derived rows", table.file()),
        }),
    }
}

pub(super) fn fold_rows<T>(table: Table, values: &[Vec<u8>]) -> StoreResult<Vec<u8>>
where
    T: Entity,
{
    let mut decoded = values.iter().map(|value| {
        serde_json::from_slice::<T>(value).map_err(|source| StoreError::Json {
            detail: format!(
                "table {} holds a row that does not decode during migration",
                table.file()
            ),
            source,
        })
    });
    let mut merged = decoded.next().ok_or_else(|| StoreError::Invariant {
        detail: format!("table {} collapsed an id with no rows", table.file()),
    })??;
    for record in decoded {
        merged.merge(record?);
    }
    serde_json::to_vec(&merged).map_err(|source| StoreError::Json {
        detail: format!("table {} could not encode a migrated row", table.file()),
        source,
    })
}

pub(super) fn drive_prefix<F>(
    db: &Database,
    entities: &Keyspace,
    prefix: &[u8],
    mut step: F,
) -> StoreResult<()>
where
    F: FnMut(&mut OwnedWriteBatch, &[u8], &[u8]) -> StoreResult<()>,
{
    let mut cursor: Option<Vec<u8>> = None;
    loop {
        let start = match &cursor {
            Some(last) => last.clone(),
            None => prefix.to_vec(),
        };
        let mut batch = db.batch();
        let mut staged = 0_usize;
        let mut last_key: Option<Vec<u8>> = None;
        let mut finished = true;
        for guard in entities.range((Bound::Excluded(start), Bound::Unbounded)) {
            let (key, value) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            if !key.starts_with(prefix) {
                break;
            }
            step(&mut batch, &key, &value)?;
            last_key = Some(key.to_vec());
            staged = staged.saturating_add(1);
            if staged >= MIGRATION_BATCH_ROWS {
                finished = false;
                break;
            }
        }
        if staged == 0 {
            break;
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        cursor = last_key;
        if finished {
            break;
        }
    }
    Ok(())
}

pub(super) fn count_prefix(entities: &Keyspace, prefix: &[u8]) -> StoreResult<u64> {
    let mut count = 0_u64;
    for guard in entities.prefix(prefix) {
        guard.key().map_err(|source| StoreError::Read { source })?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(count)
}
