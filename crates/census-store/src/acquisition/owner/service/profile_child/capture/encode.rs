use super::{refused, MAX_BYTES};
use crate::StoreResult;
use serde::Serialize;
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
            .ok_or_else(|| io::Error::other("profile capture size overflow"))?;
        if total > MAX_BYTES {
            return Err(io::Error::other("profile capture packet limit"));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| io::Error::other("profile capture allocation"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn packet<T: Serialize + ?Sized>(value: &T) -> StoreResult<Vec<u8>> {
    let mut output = Bounded { bytes: Vec::new() };
    census_domain::model::write_serialized(&mut output, value)
        .map_err(|_| refused("profile capture bounded encoding"))?;
    Ok(output.bytes)
}
