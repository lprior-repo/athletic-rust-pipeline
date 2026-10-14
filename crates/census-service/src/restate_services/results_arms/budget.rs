use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use census_domain::model::SourceMeetRef;
use census_store::{StoreError, StoreResult, MAX_ROWS_PER_TABLE};

pub(super) const CAPTURE_ROWS: u64 = MAX_ROWS_PER_TABLE;
pub(super) const CAPTURE_BYTES: u64 = 13_958_643_712;
pub(super) const SELECTION_ROWS: u64 = 65_536;
pub(super) const GLOBAL_ROWS: u64 = 3_211_264;
pub(super) const GLOBAL_BYTES: u64 = 1_073_741_824;
pub(super) const VECTOR_CHUNK: usize = 256;

mod spill;
pub(super) use spill::{DiskLease, ScratchPermit, SpillAdmission};

pub(super) const SPILL_FILES: usize = 51;
pub(super) const SPILL_BUFFER_BYTES: usize = 64 * 1024;
pub(super) const SPILL_RECORD_BYTES: usize = 6 * census_store::MAX_OBSERVATION_RECORD_BYTES + 4096;
pub(super) const SPILL_SCRATCH_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const SOURCE_SCAN_SCRATCH_BYTES: u64 = 32 * 1024 * 1024;
pub(super) const SPILL_DECODE_SCRATCH_BYTES: u64 = 25 * 1024 * 1024;
pub(super) const SPILL_METADATA_BYTES: u64 = 1024 * 1024;
pub(super) const SPILL_WRITE_CALLS: u64 = CAPTURE_BYTES / 4096 + 52_224;

#[derive(Default)]
pub(super) struct Admission {
    rows: AtomicU64,
    bytes: AtomicU64,
    pub(super) spill: Arc<SpillAdmission>,
}

pub(super) struct Permit {
    admission: Arc<Admission>,
    rows: u64,
    bytes: u64,
}

impl Admission {
    #[cfg(test)]
    pub(super) fn selected_usage(&self) -> (u64, u64) {
        (
            self.rows.load(Ordering::Acquire),
            self.bytes.load(Ordering::Acquire),
        )
    }
}

impl Permit {
    pub(super) fn new(admission: Arc<Admission>) -> Self {
        Self {
            admission,
            rows: 0,
            bytes: 0,
        }
    }

    pub(super) fn reserve(&mut self, rows: u64, bytes: u64) -> StoreResult<()> {
        let next_rows = charge(self.rows, rows, SELECTION_ROWS, "results selected rows")?;
        let next_bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        reserve(
            &self.admission.rows,
            rows,
            GLOBAL_ROWS,
            "results global selected rows",
        )?;
        if let Err(error) = reserve(
            &self.admission.bytes,
            bytes,
            GLOBAL_BYTES,
            "results global selected bytes",
        ) {
            release(&self.admission.rows, rows);
            return Err(error);
        }
        self.rows = next_rows;
        self.bytes = next_bytes;
        Ok(())
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        release(&self.admission.rows, self.rows);
        release(&self.admission.bytes, self.bytes);
    }
}

fn reserve(counter: &AtomicU64, added: u64, limit: u64, what: &'static str) -> StoreResult<()> {
    match counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
        current.checked_add(added).filter(|needed| *needed <= limit)
    }) {
        Ok(_) => Ok(()),
        Err(current) => Err(StoreError::BudgetExceeded {
            what,
            needed: current
                .checked_add(added)
                .ok_or(StoreError::CounterOverflow)?,
            available: limit,
        }),
    }
}

fn release(counter: &AtomicU64, amount: u64) {
    if counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            current.checked_sub(amount)
        })
        .is_err()
    {
        tracing::error!(amount, "results admission accounting underflow");
    }
}

pub(super) fn charge(
    current: u64,
    added: u64,
    available: u64,
    what: &'static str,
) -> StoreResult<u64> {
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(needed)
}

pub(super) fn heap_bytes(row: &SourceMeetRef) -> StoreResult<u64> {
    let fields = [
        &row.id,
        &row.source,
        &row.source_meet_id,
        &row.season,
        &row.name,
        &row.venue,
        &row.results_url,
        &row.observed_on,
    ];
    let bytes = fields.into_iter().try_fold(0_usize, |total, field| {
        total
            .checked_add(field.capacity())
            .ok_or(StoreError::CounterOverflow)
    })?;
    let bytes = bytes
        .checked_add(row.date.as_ref().map_or(0, String::capacity))
        .ok_or(StoreError::CounterOverflow)?;
    u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)
}

pub(super) fn slots<T>(count: usize) -> StoreResult<u64> {
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(StoreError::CounterOverflow)?;
    u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)
}

pub(super) fn allocation() -> StoreError {
    StoreError::Refused {
        detail: "results dataset allocation failed".to_string(),
    }
}

pub(super) fn copy(raw: &str) -> StoreResult<String> {
    let mut value = String::new();
    value
        .try_reserve_exact(raw.len())
        .map_err(|_| allocation())?;
    value.push_str(raw);
    Ok(value)
}

pub(super) fn copy_meet(row: &SourceMeetRef) -> StoreResult<SourceMeetRef> {
    Ok(SourceMeetRef {
        id: copy(&row.id)?,
        source: copy(&row.source)?,
        source_meet_id: copy(&row.source_meet_id)?,
        jurisdiction: row.jurisdiction,
        season: copy(&row.season)?,
        year: row.year,
        name: copy(&row.name)?,
        date: row.date.as_deref().map(copy).transpose()?,
        venue: copy(&row.venue)?,
        results_url: copy(&row.results_url)?,
        observed_on: copy(&row.observed_on)?,
    })
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
