use super::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub(super) const MAX_BODY: usize = 32 * 1024 * 1024;
pub(super) const MAX_META: usize = 64 * 1024;
pub(super) const MAX_ROWS: usize = 4096;

pub(super) fn read(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let size = usize::try_from(metadata.len())?;
    if !metadata.is_file() || size > maximum {
        return Err(format!("{} is not a bounded regular file", path.display()).into());
    }
    let capacity = size.checked_add(1).ok_or("read size overflow")?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity)?;
    file.take(u64::try_from(capacity)?)
        .read_to_end(&mut bytes)?;
    if bytes.len() != size {
        return Err(format!("{} changed during bounded read", path.display()).into());
    }
    Ok(bytes)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_BODY {
        return Err(format!("{} exceeds artifact bound", path.display()).into());
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn json(path: &Path, value: &impl Serialize) -> Result<()> {
    write(path, &serde_json::to_vec_pretty(value)?)
}

pub(super) fn directory(path: &Path) -> Result<()> {
    std::fs::create_dir(path)?;
    Ok(())
}
