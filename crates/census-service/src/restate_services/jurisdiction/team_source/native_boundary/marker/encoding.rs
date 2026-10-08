use std::io::{self, Write};

use serde::Serialize;

use super::BoundaryError;

struct Bounded {
    bytes: Vec<u8>,
    max: usize,
}

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("native marker byte counter overflow"))?;
        if next > self.max {
            return Err(io::Error::other(
                "native marker exceeds its explicit byte ceiling",
            ));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode(value: &impl Serialize, max: usize) -> Result<Vec<u8>, BoundaryError> {
    let mut output = Bounded {
        bytes: Vec::new(),
        max,
    };
    serde_json::to_writer(&mut output, value)?;
    Ok(output.bytes)
}
