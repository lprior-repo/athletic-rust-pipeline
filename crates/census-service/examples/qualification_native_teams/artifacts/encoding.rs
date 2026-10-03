use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{self, Write};

use super::MAX_ARTIFACT;

#[derive(Default)]
struct BoundedEncoding(Vec<u8>);

impl Write for BoundedEncoding {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length =
            self.0.len().checked_add(bytes.len()).ok_or_else(|| {
                io::Error::other("artifact resource-limit: encoded size overflow")
            })?;
        if u64::try_from(length).map_err(io::Error::other)? > MAX_ARTIFACT {
            return Err(io::Error::other(
                "artifact resource-limit: encoded size exceeds 32 MiB",
            ));
        }
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn pretty(value: &Value) -> Result<Vec<u8>> {
    let mut output = BoundedEncoding::default();
    serde_json::to_writer_pretty(&mut output, value).context("encoding bounded artifact")?;
    Ok(output.0)
}

pub(super) fn record(value: &Value) -> Result<Vec<u8>> {
    let mut output = BoundedEncoding::default();
    serde_json::to_writer(&mut output, value).context("encoding bounded evidence record")?;
    output.write_all(b"\n")?;
    Ok(output.0)
}
