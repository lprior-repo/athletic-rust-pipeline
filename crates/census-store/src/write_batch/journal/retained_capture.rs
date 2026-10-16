use std::io::{self, Write};

use census_domain::evidence_subject_attachment::AttachmentDigestV1;
use serde::Serialize;
use serde_json::Value;

use super::{checked_key, envelope};
use crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES;
use crate::{RetainedCaptureRootSchemaV1, Store, StoreError, StoreResult};

const TIMESTAMP_ALLOWANCE: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

impl Store {
    pub fn check_retained_capture_entry_v1(
        schema: RetainedCaptureRootSchemaV1,
        key: &str,
        payload: &Value,
    ) -> StoreResult<()> {
        AttachmentDigestV1::parse(key)
            .map_err(crate::EvidenceSubjectAttachmentStoreError::Domain)?;
        checked_key(schema.phase(), key)?;
        let overhead = envelope_overhead(key)?;
        measure(payload, overhead).map(|_| ())
    }
}

fn envelope_overhead(key: &str) -> StoreResult<usize> {
    let mut entry = envelope(key, Value::Null);
    let at = entry.get_mut("at").ok_or_else(|| StoreError::Invariant {
        detail: "canonical journal envelope lacks its timestamp".into(),
    })?;
    *at = Value::String(TIMESTAMP_ALLOWANCE.into());
    measure(&entry, 0)?
        .checked_sub(measure(&Value::Null, 0)?)
        .ok_or(StoreError::CounterOverflow)
}

fn measure(value: &impl Serialize, initial: usize) -> StoreResult<usize> {
    let mut counter = Counter {
        bytes: initial,
        refusal: None,
    };
    let encoded = serde_json::to_writer(&mut counter, value);
    if let Some(error) = counter.refusal {
        return Err(error);
    }
    encoded.map_err(|source| StoreError::Json {
        detail: "measuring retained capture journal envelope".into(),
        source,
    })?;
    Ok(counter.bytes)
}

struct Counter {
    bytes: usize,
    refusal: Option<StoreError>,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.refusal.is_some() {
            return Err(io::Error::other(
                "retained capture envelope already refused",
            ));
        }
        let next = self.bytes.checked_add(bytes.len());
        match next {
            Some(total) if total <= MAX_RETAINED_CAPTURE_ENVELOPE_BYTES => {
                self.bytes = total;
                Ok(bytes.len())
            }
            Some(total) => {
                self.refusal = Some(StoreError::RowTooLarge {
                    bytes: total,
                    max: MAX_RETAINED_CAPTURE_ENVELOPE_BYTES,
                });
                Err(io::Error::other(
                    "retained capture envelope exceeds its byte limit",
                ))
            }
            None => {
                self.refusal = Some(StoreError::CounterOverflow);
                Err(io::Error::other("retained capture envelope byte overflow"))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
