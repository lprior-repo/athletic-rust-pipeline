use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FileDigest {
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FileDigestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    CountConversion(#[from] std::num::TryFromIntError),
    #[error("invalid file read length")]
    InvalidReadLength,
    #[error("file byte counter exhausted")]
    CounterExhausted,
    #[error("file grew past its byte budget")]
    BudgetExceeded,
}

pub(crate) fn hash_reader(
    reader: &mut impl Read,
    max_bytes: u64,
) -> Result<FileDigest, FileDigestError> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    let mut total = 0_u64;
    for _ in 0..=max_bytes {
        let remaining = max_bytes
            .checked_sub(total)
            .ok_or(FileDigestError::BudgetExceeded)?;
        let requested = remaining
            .saturating_add(1)
            .min(u64::try_from(buffer.len())?);
        let target = buffer
            .get_mut(..usize::try_from(requested)?)
            .ok_or(FileDigestError::InvalidReadLength)?;
        let read = reader.read(target)?;
        if read == 0 {
            return Ok(FileDigest {
                bytes: total,
                sha256: format!("{:x}", digest.finalize()),
            });
        }
        let chunk = target
            .get(..read)
            .ok_or(FileDigestError::InvalidReadLength)?;
        total = total
            .checked_add(u64::try_from(read)?)
            .ok_or(FileDigestError::CounterExhausted)?;
        if total > max_bytes {
            return Err(FileDigestError::BudgetExceeded);
        }
        digest.update(chunk);
    }
    Err(FileDigestError::CounterExhausted)
}

#[cfg(test)]
mod tests;
