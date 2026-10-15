use super::super::journal::{chunk_key, chunk_value, manifest_value, split_chunks};
use serde::Serialize;

use super::super::batch::refuse_over_journal;
use super::super::{
    Store, StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
};
use super::StoreBatch;
use crate::clock::{Clock, SystemClock};

mod bounded;

pub(crate) use bounded::encode_publication_entry;

impl StoreBatch<'_> {
    pub fn journal_done<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> StoreResult<()> {
        crate::lineage::refuse_qualification_write(self.store, phase)?;
        let (row_key, value) = encode_entry(phase, key, payload)?;
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

pub(crate) fn encode_entry<T: Serialize>(
    phase: &str,
    key: &str,
    payload: &T,
) -> StoreResult<(Vec<u8>, Vec<u8>)> {
    let row_key = checked_key(phase, key)?;
    let payload = serde_json::to_value(payload).map_err(|source| StoreError::Json {
        detail: "serializing a journal payload".to_owned(),
        source,
    })?;
    let value = serde_json::to_vec(&envelope(key, payload)).map_err(|source| StoreError::Json {
        detail: "serializing a journal entry".to_owned(),
        source,
    })?;
    Ok((row_key, value))
}

fn checked_key(phase: &str, key: &str) -> StoreResult<Vec<u8>> {
    crate::journal::refuse_object_namespace(phase)?;
    crate::acquisition::codec::reject_generic_phase(phase)?;
    crate::public_identity::refuse_reserved(phase)?;
    let bytes = phase
        .len()
        .checked_add(key.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    refuse_over_journal(phase, key, "key", bytes, MAX_JOURNAL_KEY_BYTES)?;
    Ok(Store::journal_key(phase, key))
}

fn envelope(key: &str, payload: serde_json::Value) -> serde_json::Value {
    let mut entry = serde_json::Map::new();
    entry.insert("key".to_owned(), serde_json::Value::String(key.to_owned()));
    entry.insert(
        "at".to_owned(),
        serde_json::Value::String(SystemClock.today_iso8601()),
    );
    entry.insert("payload".to_owned(), payload);
    serde_json::Value::Object(entry)
}
