use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use census_store::read::{ReadBudget, ReadCounts};
use census_store::{Store, StoreError, StoreResult, Table, MAX_OBSERVATION_RECORD_BYTES};

use super::super::history_stage::HistoricalStageScope;
use super::budget::{self, Admission};
use super::selection::Selected;

mod spill;
pub(super) use spill::{Dataset, Visits};

#[derive(Default)]
pub(super) struct Cache {
    captured: Option<Captured>,
    admission: Arc<Admission>,
    capture_attempts: u64,
}

struct Captured {
    owner: Weak<Store>,
    generation: u64,
    dataset: Arc<Dataset>,
}

#[derive(Debug, Default)]
pub(super) struct Measurement {
    pub capture: ReadCounts,
    pub partition_visits: u64,
    pub spill_decoded_rows: u64,
    pub spill_read_bytes: u64,
    pub spill_read_calls: u64,
    pub spill_disk_bytes: u64,
    pub spill_write_calls: u64,
    pub spill_written_bytes: u64,
    pub spill_peak_scratch_bytes: u64,
    pub retained_bytes: u64,
    pub capture_attempts: u64,
}

impl Cache {
    pub(super) fn select(
        &mut self,
        store: &Arc<Store>,
        scope: HistoricalStageScope,
        cancel: &AtomicBool,
    ) -> StoreResult<(Selected, Measurement)> {
        check_cancel(cancel)?;
        let (dataset, capture) = self.dataset(store, cancel)?;
        let (selected, visits) = super::selection::partition(
            scope,
            Arc::clone(&self.admission),
            Arc::clone(&dataset),
            cancel,
        )?;
        check_cancel(cancel)?;
        self.check_generation(store)?;
        let (spill_write_calls, spill_written_bytes) = if capture.key_visits == 0 {
            (0, 0)
        } else {
            dataset.writes()
        };
        Ok((
            selected,
            Measurement {
                capture,
                partition_visits: visits.rows,
                spill_decoded_rows: visits.rows,
                spill_read_bytes: visits.bytes,
                spill_read_calls: visits.read_calls,
                spill_disk_bytes: dataset.disk_bytes,
                spill_write_calls,
                spill_written_bytes,
                spill_peak_scratch_bytes: dataset.peak_scratch(),
                retained_bytes: dataset.retained_bytes,
                capture_attempts: self.capture_attempts,
            },
        ))
    }

    fn dataset(
        &mut self,
        store: &Arc<Store>,
        cancel: &AtomicBool,
    ) -> StoreResult<(Arc<Dataset>, ReadCounts)> {
        let snapshot = store.snapshot();
        let generation = snapshot.evidence_generation();
        if let Some(captured) = &self.captured {
            if captured.owner.ptr_eq(&Arc::downgrade(store)) && captured.generation == generation {
                return Ok((Arc::clone(&captured.dataset), ReadCounts::default()));
            }
            if Arc::strong_count(&captured.dataset) != 1 {
                return Err(StoreError::BudgetExceeded {
                    what: "results live captures",
                    needed: 2,
                    available: 1,
                });
            }
        }
        self.captured = None;
        self.build(store, snapshot, generation, cancel)
    }

    fn build(
        &mut self,
        store: &Arc<Store>,
        snapshot: census_store::StoreSnapshot<'_>,
        generation: u64,
        cancel: &AtomicBool,
    ) -> StoreResult<(Arc<Dataset>, ReadCounts)> {
        let attempt = self
            .capture_attempts
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let mut scratch = budget::ScratchPermit::new(Arc::clone(&self.admission.spill));
        scratch.reserve(budget::SOURCE_SCAN_SCRATCH_BYTES)?;
        let mut build = spill::Build::new(store.root(), Arc::clone(&self.admission.spill))?;
        self.capture_attempts = attempt;
        let result = snapshot.for_each_merged_bounded(
            Table::SourceMeets,
            ReadBudget {
                rows: budget::CAPTURE_ROWS,
                bytes: budget::CAPTURE_BYTES,
                record_bytes: MAX_OBSERVATION_RECORD_BYTES,
            },
            |row, _encoded| build.push(row, cancel),
        );
        let counts = match result {
            Ok(counts) => counts,
            Err(primary) => return Err(build.abort(primary)),
        };
        let dataset = build.finish(cancel)?;
        drop(scratch);
        check_cancel(cancel)?;
        check_generation(store, generation)?;
        let dataset = Arc::new(dataset);
        self.captured = Some(Captured {
            owner: Arc::downgrade(store),
            generation,
            dataset: Arc::clone(&dataset),
        });
        Ok((dataset, counts))
    }

    fn check_generation(&self, store: &Store) -> StoreResult<()> {
        let captured = self.captured.as_ref().ok_or_else(budget::allocation)?;
        check_generation(store, captured.generation)
    }
}

pub(super) fn check_cancel(cancel: &AtomicBool) -> StoreResult<()> {
    if cancel.load(Ordering::Acquire) {
        return Err(StoreError::Refused {
            detail: "results capture or selection cancelled".to_string(),
        });
    }
    Ok(())
}

fn check_generation(store: &Store, expected: u64) -> StoreResult<()> {
    let actual = store.snapshot().evidence_generation();
    if actual != expected {
        return Err(StoreError::EvidenceMoved {
            operation: "results capture".to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "generation_tests.rs"]
mod tests;
