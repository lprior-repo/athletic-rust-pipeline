use std::sync::{Arc, Weak};

use census_domain::{model::SourceMeetRef, UsJurisdiction};
use census_store::read::{ReadBudget, ReadCounts};
use census_store::{Store, StoreError, StoreResult, Table, MAX_OBSERVATION_RECORD_BYTES};

use super::super::history_stage::HistoricalStageScope;
use super::budget::{self, Admission};
use super::selection::Selected;

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

pub(super) struct Dataset {
    partitions: [(UsJurisdiction, Vec<SourceMeetRef>); 51],
    decoded_bytes: u64,
}

#[derive(Debug, Default)]
pub(super) struct Measurement {
    pub capture: ReadCounts,
    pub partition_visits: u64,
    pub retained_bytes: u64,
    pub capture_attempts: u64,
}

impl Cache {
    pub(super) fn select(
        &mut self,
        store: &Arc<Store>,
        scope: HistoricalStageScope,
    ) -> StoreResult<(Selected, Measurement)> {
        let (dataset, capture) = self.dataset(store)?;
        let rows = dataset.partition(scope.jurisdiction)?;
        let (selected, partition_visits) = super::selection::partition(
            rows,
            scope,
            Arc::clone(&self.admission),
            Arc::clone(&dataset),
        )?;
        self.check_generation(store)?;
        Ok((
            selected,
            Measurement {
                capture,
                partition_visits,
                retained_bytes: dataset.decoded_bytes,
                capture_attempts: self.capture_attempts,
            },
        ))
    }

    fn dataset(&mut self, store: &Arc<Store>) -> StoreResult<(Arc<Dataset>, ReadCounts)> {
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
        let mut dataset = Dataset::new()?;
        self.capture_attempts = self
            .capture_attempts
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let counts = snapshot.for_each_merged_bounded(
            Table::SourceMeets,
            ReadBudget {
                rows: budget::CAPTURE_ROWS,
                bytes: budget::CAPTURE_BYTES,
                record_bytes: MAX_OBSERVATION_RECORD_BYTES,
            },
            |row, encoded| dataset.push(row, encoded),
        )?;
        dataset.total_bytes(counts.encoded_bytes)?;
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

impl Dataset {
    fn new() -> StoreResult<Self> {
        Ok(Self {
            partitions: UsJurisdiction::ALL.map(|jurisdiction| (jurisdiction, Vec::new())),
            decoded_bytes: budget::slots::<Self>(1)?,
        })
    }

    fn push(&mut self, row: SourceMeetRef, encoded: u64) -> StoreResult<()> {
        let rows = self
            .partitions
            .iter_mut()
            .find(|(jurisdiction, _)| *jurisdiction == row.jurisdiction)
            .map(|(_, rows)| rows)
            .ok_or_else(budget::allocation)?;
        let growth = if rows.len() == rows.capacity() {
            budget::VECTOR_CHUNK
        } else {
            0
        };
        let heap = budget::heap_bytes(&row)?;
        let added = heap
            .checked_add(budget::slots::<SourceMeetRef>(growth)?)
            .ok_or(StoreError::CounterOverflow)?;
        let decoded = self
            .decoded_bytes
            .checked_add(added)
            .ok_or(StoreError::CounterOverflow)?;
        budget::charge(
            decoded,
            encoded,
            budget::CAPTURE_BYTES,
            "results captured working bytes",
        )?;
        let old_capacity = rows.capacity();
        if growth != 0 {
            rows.try_reserve_exact(growth)
                .map_err(|_| budget::allocation())?;
        }
        let actual_growth = rows
            .capacity()
            .checked_sub(old_capacity)
            .ok_or(StoreError::CounterOverflow)?;
        let actual_slots = budget::slots::<SourceMeetRef>(actual_growth)?;
        self.decoded_bytes = self
            .decoded_bytes
            .checked_add(heap)
            .and_then(|bytes| bytes.checked_add(actual_slots))
            .ok_or(StoreError::CounterOverflow)?;
        budget::charge(
            self.decoded_bytes,
            encoded,
            budget::CAPTURE_BYTES,
            "results captured working bytes",
        )?;
        rows.push(row);
        Ok(())
    }

    fn total_bytes(&self, encoded: u64) -> StoreResult<u64> {
        budget::charge(
            self.decoded_bytes,
            encoded,
            budget::CAPTURE_BYTES,
            "results captured working bytes",
        )
    }

    fn partition(&self, jurisdiction: UsJurisdiction) -> StoreResult<&[SourceMeetRef]> {
        self.partitions
            .iter()
            .find(|(owner, _)| *owner == jurisdiction)
            .map(|(_, rows)| rows.as_slice())
            .ok_or_else(budget::allocation)
    }
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
