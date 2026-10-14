use census_domain::model::SourceMeetRef;
use census_store::{StoreError, StoreResult};
use serde::{Deserialize, Serialize};

use super::{owner, RowCharge};
use crate::restate_services::results_arms::budget;

#[derive(Serialize)]
pub(super) struct Borrowed<'a> {
    position: u64,
    row: &'a SourceMeetRef,
    charge: RowCharge,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned {
    pub(super) position: u64,
    pub(super) row: SourceMeetRef,
    pub(super) charge: RowCharge,
}

impl<'a> Borrowed<'a> {
    pub(super) fn new(row: &'a SourceMeetRef, position: u64) -> StoreResult<Self> {
        Ok(Self {
            position,
            row,
            charge: RowCharge {
                meet_heap: budget::heap_bytes(row)?,
                pending_heap: u64::try_from(row.results_url.capacity())
                    .map_err(|_| StoreError::CounterOverflow)?,
            },
        })
    }
}

impl RowCharge {
    pub(super) fn validate(self, row: &SourceMeetRef) -> StoreResult<()> {
        let minimum = [
            &row.id,
            &row.source,
            &row.source_meet_id,
            &row.season,
            &row.name,
            &row.venue,
            &row.results_url,
            &row.observed_on,
        ]
        .into_iter()
        .try_fold(0_usize, |bytes, field| {
            bytes
                .checked_add(field.len())
                .ok_or(StoreError::CounterOverflow)
        })?
        .checked_add(row.date.as_ref().map_or(0, String::len))
        .ok_or(StoreError::CounterOverflow)?;
        if self.meet_heap < u64::try_from(minimum).map_err(|_| StoreError::CounterOverflow)?
            || self.meet_heap > budget::SOURCE_SCAN_SCRATCH_BYTES
            || self.pending_heap
                < u64::try_from(row.results_url.len()).map_err(|_| StoreError::CounterOverflow)?
            || self.pending_heap > self.meet_heap
        {
            return Err(owner::refused("results spill selection charge is corrupt"));
        }
        Ok(())
    }
}
