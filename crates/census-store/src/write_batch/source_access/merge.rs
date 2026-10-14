use super::{admit_counts, page_counts, refused, ObservationBudget, Page};
use crate::keys::{observation_key, DERIVED_SEQUENCE};
use crate::{Entity, Store, StoreError, StoreResult, Table, MAX_OBSERVATION_RECORD_BYTES};
use census_domain::model::SourceAccessCondition;
use fjall::{OwnedWriteBatch, Readable};
use std::collections::hash_map::{Entry, HashMap};

struct Merged {
    record: SourceAccessCondition,
    bytes: usize,
}
struct Fold {
    rows: HashMap<String, Merged>,
    bytes: usize,
    page_rows: usize,
    page_bytes: usize,
    added: u64,
}

pub(in crate::write_batch) fn stage(
    records: Vec<(String, Vec<u8>)>,
    pages: &[Page],
    store: &Store,
    batch: &mut OwnedWriteBatch,
    conditional_capacity: Option<usize>,
) -> StoreResult<usize> {
    if records.is_empty() {
        return Ok(0);
    }
    let (page_rows, page_bytes) = page_counts(pages, &[])?;
    let mut rows = HashMap::new();
    rows.try_reserve(records.len())
        .map_err(|_| refused("allocating source access merge membership"))?;
    let mut merged = Fold {
        rows,
        bytes: 0,
        page_rows,
        page_bytes,
        added: 0,
    };
    records
        .into_iter()
        .try_for_each(|(id, value)| merged.accept(store, id, &value))?;
    let encoded = merged.rows.iter().try_fold(0usize, |total, (id, row)| {
        total
            .checked_add(id.len())
            .and_then(|total| total.checked_add(row.bytes))
            .ok_or(StoreError::CounterOverflow)
    })?;
    if conditional_capacity.is_some_and(|available| encoded > available) {
        return Err(refused("conditional staged bytes exceed 33554432"));
    }
    let count = merged.rows.len();
    let held = store.count(Table::SourceAccess)?;
    let rows = held
        .checked_add(merged.added)
        .ok_or(StoreError::CounterOverflow)?;
    crate::batch::refuse_over_bound(Table::SourceAccess, rows)?;
    let mut remaining = super::super::MAX_OBSERVATION_BATCH_BYTES
        .checked_sub(page_bytes)
        .ok_or(StoreError::CounterOverflow)?;
    merged.rows.into_iter().try_for_each(|(id, row)| {
        let value = super::super::budget::encode(&row.record, remaining)?;
        remaining = remaining
            .checked_sub(value.len())
            .ok_or(StoreError::CounterOverflow)?;
        batch.insert(
            &store.entities,
            observation_key(Table::SourceAccess, &id, DERIVED_SEQUENCE),
            value,
        );
        Ok::<_, StoreError>(())
    })?;
    store.put_row_mark(batch, Table::SourceAccess, rows);
    Ok(count)
}

impl Fold {
    fn accept(&mut self, store: &Store, id: String, value: &[u8]) -> StoreResult<()> {
        let incoming = decode(value)?;
        let prior_rows = self.rows.len();
        let (row, old_bytes, count) = match self.rows.entry(id) {
            Entry::Occupied(mut held) => {
                let prior = held.get().bytes;
                held.get_mut().record.merge(incoming);
                (held.into_mut(), prior, prior_rows)
            }
            Entry::Vacant(held) => {
                let record = match read(store, held.key())? {
                    Some(mut retained) => {
                        retained.merge(incoming);
                        retained
                    }
                    None => {
                        self.added = self
                            .added
                            .checked_add(1)
                            .ok_or(StoreError::CounterOverflow)?;
                        incoming
                    }
                };
                (
                    held.insert(Merged { record, bytes: 0 }),
                    0,
                    prior_rows
                        .checked_add(1)
                        .ok_or(StoreError::CounterOverflow)?,
                )
            }
        };
        let bytes = ObservationBudget::record_bytes(&row.record)?;
        let total = self
            .bytes
            .checked_sub(old_bytes)
            .and_then(|total| total.checked_add(bytes))
            .ok_or(StoreError::CounterOverflow)?;
        admit_counts(
            self.page_rows
                .checked_add(count)
                .ok_or(StoreError::CounterOverflow)?,
            self.page_bytes
                .checked_add(total)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        row.bytes = bytes;
        self.bytes = total;
        Ok(())
    }
}

fn decode(value: &[u8]) -> StoreResult<SourceAccessCondition> {
    if value.len() > MAX_OBSERVATION_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: value.len(),
            max: MAX_OBSERVATION_RECORD_BYTES,
        });
    }
    serde_json::from_slice(value).map_err(|source| StoreError::Json {
        detail: "decoding retained source access condition".to_string(),
        source,
    })
}
fn read(store: &Store, id: &str) -> StoreResult<Option<SourceAccessCondition>> {
    let key = observation_key(Table::SourceAccess, id, DERIVED_SEQUENCE);
    let snapshot = store.db.snapshot();
    if let Some(length) = snapshot
        .size_of(&store.entities, &key)
        .map_err(|source| StoreError::Read { source })?
    {
        let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if bytes > MAX_OBSERVATION_RECORD_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes,
                max: MAX_OBSERVATION_RECORD_BYTES,
            });
        }
    }
    #[cfg(test)]
    super::tests::point_read()?;
    let held = snapshot
        .get(&store.entities, key)
        .map_err(|source| StoreError::Read { source })?;
    held.map(|value| {
        let record = decode(&value)?;
        if record.entity_id() != id {
            return Err(StoreError::Invariant {
                detail: "source access key disagrees with retained id".to_string(),
            });
        }
        Ok(record)
    })
    .transpose()
}
