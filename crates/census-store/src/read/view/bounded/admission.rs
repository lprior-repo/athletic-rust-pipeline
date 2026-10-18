use super::{
    charge, decode_physical_observation, invariant, read_admitted, PhysicalScan,
    PhysicalScanResult, ReadBudget, ReadCounts, StoreSnapshot,
};
use crate::{Entity, StoreError, StoreResult, Table};
use fjall::Readable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalReadAdmissionV1 {
    KeyProbe,
    ValueSizeProbe,
    Value {
        key_bytes: usize,
        value_bytes: usize,
    },
}

impl StoreSnapshot<'_> {
    pub fn for_each_physical_observation_admitted<T: Entity, C>(
        &self,
        table: Table,
        budget: ReadBudget,
        context: &mut C,
        admit: impl FnMut(&mut C, PhysicalReadAdmissionV1) -> StoreResult<()>,
        visit: impl FnMut(&mut C, T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        Ok(self
            .scan_physical_admitted(table, budget, PhysicalScan::Whole, context, admit, visit)?
            .counts)
    }

    pub(super) fn scan_physical_admitted<T: Entity, C>(
        &self,
        table: Table,
        budget: ReadBudget,
        mode: PhysicalScan<'_>,
        context: &mut C,
        mut admit: impl FnMut(&mut C, PhysicalReadAdmissionV1) -> StoreResult<()>,
        mut visit: impl FnMut(&mut C, T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<PhysicalScanResult> {
        let mut counts = ReadCounts::default();
        let prefix = self.layout().prefix(table);
        admit(context, PhysicalReadAdmissionV1::KeyProbe)?;
        let mut rows = mode.rows(self, &prefix);
        loop {
            admit(context, PhysicalReadAdmissionV1::KeyProbe)?;
            let Some(guard) = rows.next() else {
                break;
            };
            if matches!(mode, PhysicalScan::Whole) {
                charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            }
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if !key.starts_with(&prefix) {
                break;
            }
            if matches!(mode, PhysicalScan::Page(_))
                && counts.key_visits == budget.rows
                && counts.decoded_rows > 0
            {
                return Ok(next_page(counts, key));
            }
            let mut admitted = counts;
            admitted.key_visits =
                charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            let raw =
                self.load_physical_observation(&key, &mut admitted, budget, context, &mut admit);
            let raw = match raw {
                Err(error) if mode.can_continue(&error, counts, budget) => {
                    return Ok(next_page(counts, key));
                }
                result => result?,
            };
            let row = decode_physical_observation(table, &key, &raw)?;
            admitted.decoded_rows = counts
                .decoded_rows
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            visit(context, row, &key, &raw, admitted.encoded_bytes)?;
            counts = admitted;
        }
        Ok(PhysicalScanResult {
            counts,
            next_key: None,
        })
    }

    fn load_physical_observation<C>(
        &self,
        key: &[u8],
        counts: &mut ReadCounts,
        budget: ReadBudget,
        context: &mut C,
        admit: &mut impl FnMut(&mut C, PhysicalReadAdmissionV1) -> StoreResult<()>,
    ) -> StoreResult<fjall::UserValue> {
        admit(context, PhysicalReadAdmissionV1::ValueSizeProbe)?;
        let size = self
            .snapshot
            .size_of(self.entities, key)
            .map_err(|source| StoreError::Read { source })?;
        read_admitted(key.len(), size, counts, budget, || {
            let value_bytes = usize::try_from(
                size.ok_or_else(|| invariant("bounded observation size is missing"))?,
            )
            .map_err(|_| StoreError::CounterOverflow)?;
            admit(
                context,
                PhysicalReadAdmissionV1::Value {
                    key_bytes: key.len(),
                    value_bytes,
                },
            )?;
            self.snapshot
                .get(self.entities, key)
                .map_err(|source| StoreError::Read { source })
        })
    }
}

fn next_page(counts: ReadCounts, key: fjall::UserKey) -> PhysicalScanResult {
    PhysicalScanResult {
        counts,
        next_key: Some(key),
    }
}
