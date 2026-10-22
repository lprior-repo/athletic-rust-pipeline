use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{self, Write};

struct LimitBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for LimitBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let needed = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("inventory output length overflow"))?;
        if needed > self.limit {
            return Err(io::Error::other("inventory output byte budget exceeded"));
        }
        if needed > self.bytes.capacity() {
            let target = needed
                .checked_next_power_of_two()
                .ok_or_else(|| io::Error::other("inventory buffer capacity overflow"))?
                .min(self.limit);
            let additional = target
                .checked_sub(self.bytes.len())
                .ok_or_else(|| io::Error::other("inventory buffer capacity underflow"))?;
            self.bytes
                .try_reserve_exact(additional)
                .map_err(|_| io::Error::other("inventory output allocation refused"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn emit<W: Write, T: Serialize>(output: &mut W, value: &T, limit: usize) -> Result<()> {
    let mut buffer = LimitBuffer {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut buffer, value).context("encoding bounded roster inventory")?;
    buffer.write_all(b"\n")?;
    output
        .write_all(&buffer.bytes)
        .context("writing complete unreviewed roster inventory")
}
