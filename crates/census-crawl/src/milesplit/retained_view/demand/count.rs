use sha2::{Digest, Sha256};
use std::io;

pub(super) const MAX_CANONICAL_BYTES: u64 = 34_359_738_368;

pub(super) struct CanonicalCount {
    bytes: u64,
    hash: Sha256,
}

pub(super) fn charge(current: u64, additional: usize) -> io::Result<u64> {
    let add = u64::try_from(additional).map_err(|_| io::Error::other("counter overflow"))?;
    let sum = current
        .checked_add(add)
        .ok_or_else(|| io::Error::other("counter overflow"))?;
    if sum > MAX_CANONICAL_BYTES {
        return Err(io::Error::other("byte budget exceeded"));
    }
    Ok(sum)
}

impl CanonicalCount {
    pub(super) fn new() -> Self {
        Self {
            bytes: 0,
            hash: Sha256::new(),
        }
    }

    pub(super) fn bytes(&self) -> u64 {
        self.bytes
    }

    pub(super) fn finish(self) -> (u64, String) {
        (self.bytes, format!("{:x}", self.hash.finalize()))
    }
}

impl io::Write for CanonicalCount {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = charge(self.bytes, bytes.len())?;
        self.hash.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
