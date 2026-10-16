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
        let value_bytes = self
            .source
            .size(key)?
            .ok_or_else(|| self.path().refusal("retained physical row is absent"))?;
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

impl JournalObject<'_> {
    pub(crate) fn for_each_physical_row(
        &self,
        expected_bytes: u64,
        mut visit: impl FnMut(&[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        if self.retained_physical_bytes(expected_bytes)? != expected_bytes {
            return Err(self
                .path()
                .refusal("retained physical bytes differ from reference"));
        }
        self.visit_physical_row(&object_manifest_key(self.object_id())?, &mut visit)?;
        for index in 0..self.bounds.index_pages {
            self.visit_physical_row(&object_index_key(self.object_id(), index)?, &mut visit)?;
        }
        for index in 0..self.chunk_count() {
            self.visit_physical_row(&object_chunk_key(self.object_id(), index)?, &mut visit)?;
        }
        Ok(())
    }

    fn visit_physical_row(
        &self,
        key: &[u8],
        visit: &mut impl FnMut(&[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let raw = self
            .get(key)?
            .ok_or_else(|| self.path().refusal("retained physical row is absent"))?;
        visit(key, raw.as_ref())
    }
}
