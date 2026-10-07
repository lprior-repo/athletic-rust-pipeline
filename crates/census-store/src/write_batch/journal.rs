use super::super::journal::{chunk_key, chunk_value, manifest_value, split_chunks};
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
        if value.len() <= MAX_JOURNAL_VALUE_BYTES {
            self.journal.push((row_key, value));
            Ok(())
        } else {
            let raw_chunks = split_chunks(&value);
            let chunk_total = raw_chunks.len();
            let wrapped: Vec<Vec<u8>> = raw_chunks
                .into_iter()
                .enumerate()
                .map(|(index, raw)| chunk_value(key, index, chunk_total, &raw))
                .collect::<Result<_, StoreError>>()?;
            for piece in &wrapped {
                if piece.len() > MAX_JOURNAL_VALUE_BYTES {
                    return refuse_over_journal(
                        phase,
                        key,
                        "value",
                        piece.len(),
                        MAX_JOURNAL_VALUE_BYTES,
                    );
                }
            }
            let manifest = manifest_value(key, chunk_total, value.len())?;
            self.journal.push((row_key.clone(), manifest));
            for (index, piece) in wrapped.into_iter().enumerate() {
                self.journal.push((chunk_key(&row_key, index)?, piece));
            }
            Ok(())
        }
    }
}
