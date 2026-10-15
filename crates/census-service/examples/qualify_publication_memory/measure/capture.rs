use super::OUTPUT_LIMIT;
use anyhow::{Context, Result};
use std::io::{self, Read};

#[derive(Default)]
pub(super) struct Capture {
    pub(super) bytes: Vec<u8>,
    pub(super) exceeded: bool,
    pub(super) eof: bool,
}

impl Capture {
    pub(super) fn poll(&mut self, input: &mut impl Read) -> Result<()> {
        let mut bytes = [0_u8; 8192];
        for _ in 0..16 {
            match input.read(&mut bytes) {
                Ok(0) => {
                    self.eof = true;
                    break;
                }
                Ok(n) => {
                    let remaining = OUTPUT_LIMIT.saturating_sub(self.bytes.len());
                    self.bytes
                        .extend_from_slice(bytes.get(..n.min(remaining)).context("output extent")?);
                    self.exceeded |= n > remaining;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    break
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
