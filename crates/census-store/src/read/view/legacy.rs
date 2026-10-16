use serde_json::{value::RawValue, Value};

use super::legacy_wire;
use super::retained_source::legacy::payload_bounded;
use super::StoreSnapshot;
use crate::journal::OBJECT_RECORD_BYTES_LIMIT;
use crate::{JournalReadBudget, StoreResult};

impl StoreSnapshot<'_> {
    pub fn journal_payload(&self, phase: &str, key: &str) -> StoreResult<Option<Value>> {
        let budget = JournalReadBudget::new(OBJECT_RECORD_BYTES_LIMIT)?;
        self.journal_payload_bounded(phase, key, budget)
    }

    pub fn journal_payload_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        payload_bounded(self, phase, key, budget, legacy_wire::value_entry)
    }

    pub fn journal_payload_raw_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Box<RawValue>>> {
        payload_bounded(self, phase, key, budget, |path, what, raw| {
            super::legacy_raw::entry(path, what, raw, budget)
        })
    }

    pub(crate) fn journal_entry_raw_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<super::JournalRawEnvelope>> {
        payload_bounded(self, phase, key, budget, |path, what, raw| {
            super::legacy_raw::envelope::entry(path, what, raw, budget)
        })
    }
}
