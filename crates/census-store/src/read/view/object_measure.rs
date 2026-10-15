use fjall::Readable;

use super::object::JournalObject;
use crate::journal::{object_chunk_key, object_index_key, object_manifest_key};
use crate::{StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

impl JournalObject<'_> {
    pub(crate) fn retained_physical_bytes(&self, maximum: u64) -> StoreResult<u64> {
        let manifest = object_manifest_key(self.object_id())?;
        let initial = self.add_physical_row(0, &manifest, maximum)?;
        let indexes = (0..self.bounds.index_pages).try_fold(initial, |total, index| {
            self.add_physical_row(total, &object_index_key(self.object_id(), index)?, maximum)
        })?;
        (0..self.chunk_count()).try_fold(indexes, |total, index| {
            self.add_physical_row(total, &object_chunk_key(self.object_id(), index)?, maximum)
        })
    }

    fn add_physical_row(&self, total: u64, key: &[u8], maximum: u64) -> StoreResult<u64> {
        let length = self
            .snapshot
            .size_of(self.journal, key)
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| self.path().refusal("retained physical row is absent"))?;
        let value_bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        super::legacy_wire::limit(
            self.path(),
            "physical object row",
            value_bytes,
            MAX_JOURNAL_VALUE_BYTES,
        )?;
        let key_bytes = u64::try_from(key.len()).map_err(|_| StoreError::CounterOverflow)?;
        let value_bytes = u64::try_from(value_bytes).map_err(|_| StoreError::CounterOverflow)?;
        let observed = total
            .checked_add(key_bytes)
            .and_then(|sum| sum.checked_add(value_bytes))
            .ok_or(StoreError::CounterOverflow)?;
        if observed > maximum {
            return Err(self
                .path()
                .refusal("retained physical bytes exceed reference"));
        }
        Ok(observed)
    }
}
