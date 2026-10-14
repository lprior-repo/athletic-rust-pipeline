use serde::Serialize;

use crate::batch::{refuse_derived_append, refuse_observation_replacement};

use super::super::batch::refuse_over_bound;
use super::super::keys::observation_id;
use super::super::{StorageMode, StoreError, StoreResult, Table};
use super::{ObservationBudget, Page, Replacement, StoreBatch};

impl StoreBatch<'_> {
    pub fn append_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        refuse_derived_append(table)?;
        if records.is_empty() {
            return Ok(());
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        refuse_over_bound(table, count)?;
        let (held_rows, held_bytes) = self.pending_counts()?;
        ObservationBudget::admit(records, held_rows, held_bytes)?;
        let mut encoded = Vec::new();
        encoded
            .try_reserve_exact(records.len())
            .map_err(|_| StoreError::Refused {
                detail: "allocating admitted observation rows".to_string(),
            })?;
        let mut remaining = super::MAX_OBSERVATION_BATCH_BYTES
            .checked_sub(held_bytes)
            .ok_or(StoreError::CounterOverflow)?;
        for record in records {
            let value = super::budget::encode(record, remaining)?;
            remaining = remaining
                .checked_sub(value.len())
                .ok_or(StoreError::CounterOverflow)?;
            encoded.push((observation_id(&value)?, value));
        }
        match self.pages.iter_mut().find(|page| page.table == table) {
            Some(page) => {
                page.records
                    .try_reserve(encoded.len())
                    .map_err(|_| StoreError::Refused {
                        detail: "allocating admitted observation page".to_string(),
                    })?;
                page.records.extend(encoded);
            }
            None => {
                self.pages.try_reserve(1).map_err(|_| StoreError::Refused {
                    detail: "allocating admitted observation table".to_string(),
                })?;
                self.pages.push(Page {
                    table,
                    records: encoded,
                });
            }
        }
        Ok(())
    }

    pub fn record_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        match table.storage_mode() {
            StorageMode::ObservationLog => self.append_many(table, records),
            StorageMode::DerivedGeneration | StorageMode::DerivedMap => {
                self.replace_many(table, records)
            }
        }
    }

    pub fn replace_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        refuse_observation_replacement(table)?;
        if records.is_empty() {
            return Ok(());
        }
        if table == Table::SourceAccess && !self.source_access.is_empty() {
            return Err(super::source_access::mixed());
        }
        if self.replacements.iter().any(|held| held.table == table) {
            return Err(StoreError::Invariant {
                detail: format!("table {} is replaced twice in one batch", table.file()),
            });
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        refuse_over_bound(table, count)?;
        let mut encoded = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing a derived record".to_string(),
                source,
            })?;
            encoded.push(value);
        }
        self.replacements.push(Replacement {
            table,
            records: encoded,
        });
        Ok(())
    }
}
