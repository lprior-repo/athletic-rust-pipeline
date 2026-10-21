use super::{refused, Wire, MAX_PACKET};
use crate::StoreResult;
use std::io::{self, Write};

struct Bounded {
    bytes: Vec<u8>,
}

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let total = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("public school intake size overflow"))?;
        if total > MAX_PACKET {
            return Err(io::Error::other("public school intake packet limit"));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| io::Error::other("public school intake allocation"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn wire(value: &Wire) -> StoreResult<Vec<u8>> {
    let mut output = Bounded { bytes: Vec::new() };
    census_domain::model::write_serialized(&mut output, value)
        .map_err(|_| refused("public school intake bounded encoding"))?;
    Ok(output.bytes)
}
