use crate::{Entity, StoreError, StoreResult, Table};

use super::bounded::{PhysicalScan, PhysicalScanResult};
use super::{ReadBudget, ReadCounts, StoreSnapshot};

pub struct PhysicalObservationCursor<'a> {
    snapshot: &'a fjall::Snapshot,
    table: Table,
    next_key: fjall::UserKey,
}

pub enum PhysicalObservationPage<'a> {
    Continue {
        counts: ReadCounts,
        cursor: PhysicalObservationCursor<'a>,
    },
    Exhausted {
        counts: ReadCounts,
    },
}

impl StoreSnapshot<'_> {
    pub(crate) fn scan_physical_observation_page_from_key<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        start: Option<&[u8]>,
        visit: impl FnMut(T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<(ReadCounts, Option<fjall::UserKey>)> {
        if start.is_some_and(|key| !key.starts_with(&self.layout().prefix(table))) {
            return Err(StoreError::Refused {
                detail: "physical observation start belongs to another table or generation".into(),
            });
        }
        let PhysicalScanResult { counts, next_key } =
            self.scan_physical_observations(table, budget, PhysicalScan::Page(start), visit)?;
        Ok((counts, next_key))
    }

    pub fn for_each_physical_observation_page<'a, T: Entity>(
        &'a self,
        table: Table,
        budget: ReadBudget,
        cursor: Option<&PhysicalObservationCursor<'_>>,
        visit: impl FnMut(T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<PhysicalObservationPage<'a>> {
        let start = match cursor {
            Some(cursor) => {
                if !std::ptr::eq(cursor.snapshot, &self.snapshot) || cursor.table != table {
                    return Err(StoreError::Refused {
                        detail: "physical observation cursor belongs to another snapshot or table"
                            .into(),
                    });
                }
                Some(cursor.next_key.as_ref())
            }
            None => None,
        };
        let PhysicalScanResult { counts, next_key } =
            self.scan_physical_observations(table, budget, PhysicalScan::Page(start), visit)?;
        Ok(match next_key {
            Some(next_key) => PhysicalObservationPage::Continue {
                counts,
                cursor: PhysicalObservationCursor {
                    snapshot: &self.snapshot,
                    table,
                    next_key,
                },
            },
            None => PhysicalObservationPage::Exhausted { counts },
        })
    }
}

#[cfg(test)]
#[path = "bounded_page_tests.rs"]
mod tests;
