use serde::Serialize;

use super::super::batch::refuse_over_journal;
use super::super::{
    Store, StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
};
use super::StoreBatch;
use crate::clock::{Clock, SystemClock};

impl StoreBatch<'_> {
    /// Record a completed unit of work in the same commit as the rows it names.
    ///
    /// The entry's `at` stamp is read when the entry is buffered, not at the commit: the producers
    /// that use this call commit the batch holding both the rows and the entry right after.
    ///
    /// The entry is bounded exactly as [`Store::journal_done`] bounds it, and the refusal names the
    /// phase and key without either reaching the batch.
    pub fn journal_done<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> StoreResult<()> {
        let entry = serde_json::json!({
            "key": key,
            "at": SystemClock.today_iso8601(),
            "payload": payload,
        });
        let value = serde_json::to_vec(&entry).map_err(|source| StoreError::Json {
            detail: "serializing a journal entry".to_string(),
            source,
        })?;
        let row_key = Store::journal_key(phase, key);
        refuse_over_journal(phase, key, "key", row_key.len(), MAX_JOURNAL_KEY_BYTES)?;
        refuse_over_journal(phase, key, "value", value.len(), MAX_JOURNAL_VALUE_BYTES)?;
        self.journal.push((row_key, value));
        Ok(())
    }
}
