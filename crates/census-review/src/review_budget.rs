use std::io::{self, Write};

use census_store::{StoreError, StoreResult};
use serde::Serialize;

pub(super) const CHECKPOINT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Budget {
    retained: usize,
}

impl Budget {
    pub(super) fn charge<T: Serialize>(&mut self, value: &T) -> StoreResult<()> {
        self.add(encoded_size(value)?)
    }

    pub(super) fn add(&mut self, bytes: usize) -> StoreResult<()> {
        let retained = self
            .retained
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        if retained > CHECKPOINT_BYTES {
            return Err(super::consensus::invariant(format!(
                "review checkpoint exceeds aggregate byte budget {CHECKPOINT_BYTES}: {retained}"
            )));
        }
        self.retained = retained;
        Ok(())
    }
}

pub(super) fn encoded_size<T: Serialize>(value: &T) -> StoreResult<usize> {
    let mut writer = Counter::default();
    serde_json::to_writer(&mut writer, value).map_err(|source| StoreError::Json {
        detail: "counting review checkpoint bytes".to_string(),
        source,
    })?;
    Ok(writer.bytes)
}

#[derive(Default)]
struct Counter {
    bytes: usize,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("review checkpoint byte counter overflow"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
