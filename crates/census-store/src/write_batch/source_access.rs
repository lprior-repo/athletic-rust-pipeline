use super::{ObservationBudget, Page, StoreBatch, MAX_OBSERVATION_BATCH_BYTES};
use crate::keys::observation_id;
use crate::{StoreError, StoreResult, Table};
use census_domain::model::SourceAccessCondition;

mod merge;
#[cfg(test)]
mod tests;

pub(super) use merge::stage;

impl StoreBatch<'_> {
    pub fn merge_source_access(&mut self, records: &[SourceAccessCondition]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        if self
            .replacements
            .iter()
            .any(|held| held.table == Table::SourceAccess)
        {
            return Err(mixed());
        }
        let (rows, bytes) = self.pending_counts()?;
        ObservationBudget::admit(records, rows, bytes)?;
        let available = MAX_OBSERVATION_BATCH_BYTES
            .checked_sub(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        let mut encoded = Vec::new();
        encoded
            .try_reserve_exact(records.len())
            .map_err(|_| refused("allocating source access staging"))?;
        let (encoded, _) =
            records
                .iter()
                .try_fold((encoded, available), |(mut encoded, left), record| {
                    let value = super::budget::encode(record, left)?;
                    let next = left
                        .checked_sub(value.len())
                        .ok_or(StoreError::CounterOverflow)?;
                    encoded.push((observation_id(&value)?, value));
                    Ok::<_, StoreError>((encoded, next))
                })?;
        self.source_access
            .try_reserve(encoded.len())
            .map_err(|_| refused("allocating pending source access rows"))?;
        self.source_access.extend(encoded);
        Ok(())
    }

    pub(super) fn pending_counts(&self) -> StoreResult<(usize, usize)> {
        page_counts(&self.pages, &self.source_access)
    }

    pub(super) fn admit_source_access(&self) -> StoreResult<()> {
        if self.source_access.is_empty() {
            return Ok(());
        }
        if self
            .replacements
            .iter()
            .any(|held| held.table == Table::SourceAccess)
        {
            return Err(mixed());
        }
        let (rows, bytes) = self.pending_counts()?;
        admit_counts(rows, bytes)
    }
}

pub(super) fn page_counts(
    pages: &[Page],
    access: &[(String, Vec<u8>)],
) -> StoreResult<(usize, usize)> {
    pages
        .iter()
        .flat_map(|page| page.records.iter())
        .chain(access)
        .try_fold((0usize, 0usize), |(rows, bytes), (_, value)| {
            Ok((
                rows.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                bytes
                    .checked_add(value.len())
                    .ok_or(StoreError::CounterOverflow)?,
            ))
        })
}

pub(super) fn admit_counts(rows: usize, bytes: usize) -> StoreResult<()> {
    ObservationBudget::admit(std::iter::empty::<&SourceAccessCondition>(), rows, bytes)?;
    Ok(())
}

pub(super) fn mixed() -> StoreError {
    StoreError::Invariant {
        detail: "source access replacement and merge occur in one batch".to_string(),
    }
}
fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}
