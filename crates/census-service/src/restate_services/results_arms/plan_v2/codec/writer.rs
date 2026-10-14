use super::super::{ResultsPlanError, MAX_PLAN_BYTES};
use std::io::{self, Write};

struct Counter {
    length: usize,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.length = self
            .length
            .checked_add(bytes.len())
            .filter(|length| *length <= MAX_PLAN_BYTES)
            .ok_or_else(|| io::Error::other("encoded plan bytes"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Output {
    bytes: Vec<u8>,
    admitted: usize,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|length| *length <= self.admitted)
            .ok_or_else(|| io::Error::other("encoding exceeded admitted count"))?;
        if length > self.bytes.capacity() {
            return Err(io::Error::other("encoding capacity changed"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(in crate::restate_services::results_arms::plan_v2) fn count<T: serde::Serialize>(
    value: &T,
) -> Result<usize, ResultsPlanError> {
    let mut counter = Counter { length: 0 };
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| ResultsPlanError::Limit("encoded plan bytes"))?;
    Ok(counter.length)
}

pub(in crate::restate_services::results_arms::plan_v2) fn encode<T: serde::Serialize>(
    value: &T,
    length: usize,
) -> Result<Vec<u8>, ResultsPlanError> {
    if length > MAX_PLAN_BYTES {
        return Err(ResultsPlanError::Limit("encoded plan bytes"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| ResultsPlanError::Allocation)?;
    let mut output = Output {
        bytes,
        admitted: length,
    };
    serde_json::to_writer(&mut output, value)?;
    if output.bytes.len() != length {
        return Err(ResultsPlanError::Invalid("encoding length changed"));
    }
    Ok(output.bytes)
}
