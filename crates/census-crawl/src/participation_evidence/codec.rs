use super::{bound, invalid, json};
use census_store::{StoreError, StoreResult};
use serde::Serialize;
use std::io::{Error, Write};

pub(super) fn encode(value: &impl Serialize, limit: usize) -> StoreResult<Vec<u8>> {
    let mut sink = Sink {
        bytes: Vec::new(),
        limit,
        failure: None,
    };
    let encoded = serde_json::to_writer(&mut sink, value);
    if let Some(error) = sink.failure {
        return Err(error);
    }
    encoded.map_err(json)?;
    Ok(sink.bytes)
}

struct Sink {
    bytes: Vec<u8>,
    limit: usize,
    failure: Option<StoreError>,
}

impl Sink {
    fn append(&mut self, value: &[u8]) -> StoreResult<()> {
        let needed = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(StoreError::CounterOverflow)?;
        bound(
            needed,
            self.limit,
            "independent participation encoded bytes",
        )?;
        self.bytes
            .try_reserve(value.len())
            .map_err(|_| invalid("participation encoding allocation refused"))?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }
}

impl Write for Sink {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        if self.failure.is_some() {
            return Err(Error::other("participation encoder is poisoned"));
        }
        match self.append(value) {
            Ok(()) => Ok(value.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(Error::other("participation bounded encoding refused"))
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
