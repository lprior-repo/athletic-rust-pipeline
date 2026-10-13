use std::collections::BTreeMap;

use fjall::OwnedWriteBatch;
use serde::Serialize;

use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

use super::{MAX_STAGE_BYTES, MAX_STAGE_ROWS};

#[derive(Default)]
pub(super) struct PublicationJournals {
    entries: BTreeMap<Vec<u8>, PendingJournal>,
    bytes: usize,
}

struct PendingJournal {
    phase: String,
    key: String,
    value: Vec<u8>,
}

impl PublicationJournals {
    pub(super) fn stage<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> StoreResult<()> {
        if phase.is_empty()
            || key.is_empty()
            || phase.chars().any(char::is_control)
            || key.chars().any(char::is_control)
        {
            return Err(StoreError::Refused {
                detail: "publication journal requires nonempty unambiguous phase and key"
                    .to_owned(),
            });
        }
        let (row_key, value) = crate::write_batch::encode_publication_entry(phase, key, payload)?;
        crate::batch::refuse_over_journal(
            phase,
            key,
            "value",
            value.len(),
            MAX_JOURNAL_VALUE_BYTES,
        )?;
        if let Some(held) = self.entries.get(&row_key) {
            return refuse_mismatch(phase, key, &held.value, &value);
        }
        let bytes = self
            .bytes
            .checked_add(row_key.len())
            .and_then(|bytes| bytes.checked_add(phase.len()))
            .and_then(|bytes| bytes.checked_add(key.len()))
            .and_then(|bytes| bytes.checked_add(value.len()))
            .ok_or(StoreError::CounterOverflow)?;
        if self.entries.len() >= MAX_STAGE_ROWS || bytes > MAX_STAGE_BYTES {
            return Err(StoreError::Refused {
                detail: "publication journals exceed the row or byte budget".to_owned(),
            });
        }
        self.entries.insert(
            row_key,
            PendingJournal {
                phase: phase.to_owned(),
                key: key.to_owned(),
                value,
            },
        );
        self.bytes = bytes;
        Ok(())
    }

    pub(super) fn write(&self, store: &Store, batch: &mut OwnedWriteBatch) -> StoreResult<()> {
        for (row_key, entry) in &self.entries {
            let raw = store
                .journal
                .get(row_key.as_slice())
                .map_err(|source| StoreError::Read { source })?;
            match raw {
                Some(held) if held.len() > MAX_JOURNAL_VALUE_BYTES => {
                    return Err(conflict(&entry.phase, &entry.key));
                }
                Some(held) => {
                    refuse_mismatch(&entry.phase, &entry.key, held.as_ref(), &entry.value)?;
                }
                None => batch.insert(&store.journal, row_key.as_slice(), entry.value.as_slice()),
            }
        }
        Ok(())
    }
}

fn payload(value: &[u8]) -> StoreResult<serde_json::Value> {
    let mut entry: serde_json::Value =
        serde_json::from_slice(value).map_err(|source| StoreError::Json {
            detail: "decoding prepared publication journal".to_owned(),
            source,
        })?;
    entry
        .as_object_mut()
        .and_then(|object| object.remove("payload"))
        .ok_or_else(|| StoreError::Invariant {
            detail: "prepared publication journal has no payload".to_owned(),
        })
}

fn refuse_mismatch(phase: &str, key: &str, held: &[u8], next: &[u8]) -> StoreResult<()> {
    if payload(held)? == payload(next)? {
        Ok(())
    } else {
        Err(conflict(phase, key))
    }
}

fn conflict(phase: &str, key: &str) -> StoreError {
    StoreError::PublicationRefused {
        detail: format!("publication journal {phase}/{key} already names a different payload"),
    }
}
