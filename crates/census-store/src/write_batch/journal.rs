use serde::Serialize;

use super::super::batch::refuse_over_journal;
use super::super::{
    Store, StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
};
use super::StoreBatch;
use crate::clock::{Clock, SystemClock};

impl StoreBatch<'_> {
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
