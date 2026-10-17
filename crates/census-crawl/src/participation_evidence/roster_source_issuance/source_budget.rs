use super::{domain, AttachmentReplayGrantV1};
use crate::participation_evidence::{bound, json, MAX_PACKET};
use census_store::{StoreError, StoreResult};
use serde::Serialize;
use std::io::{Error, Write};

pub(in crate::participation_evidence::roster_source_issuance) fn charge(value: &impl Serialize, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    grant.ensure_available().map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let bytes = measure(value)?;
    grant.charge_bytes(bytes).map_err(domain)
}

fn measure(value: &impl Serialize) -> StoreResult<usize> {
    let mut sink = Counter { bytes: 0, failure: None };
    let result = serde_json::to_writer(&mut sink, value);
    if let Some(error) = sink.failure {
        return Err(error);
    }
    result.map_err(json)?;
    Ok(sink.bytes)
}

struct Counter {
    bytes: usize,
    failure: Option<StoreError>,
}

impl Counter {
    fn count(&mut self, bytes: usize) -> StoreResult<()> {
        let total = self.bytes.checked_add(bytes).ok_or(StoreError::CounterOverflow)?;
        bound(total, MAX_PACKET, "roster source serialization measurement")?;
        self.bytes = total;
        Ok(())
    }
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.failure.is_some() {
            return Err(Error::other("roster source measurement poisoned"));
        }
        match self.count(bytes.len()) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(Error::other("roster source measurement refused"))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
