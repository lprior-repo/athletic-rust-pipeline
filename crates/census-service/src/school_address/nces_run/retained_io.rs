use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_CAPTURE_BYTES: u64 = 128 * 1024 * 1024;

pub(super) fn manifest(path: &Path) -> Result<Vec<u8>> {
    let (file, length) = open(path, MAX_MANIFEST_BYTES)?;
    let capacity = usize::try_from(length).context("retained manifest length")?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity)?;
    file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? > MAX_MANIFEST_BYTES {
        return Err(too_large().into());
    }
    if u64::try_from(bytes.len())? != length {
        bail!("retained NCES manifest changed while reading");
    }
    Ok(bytes)
}

pub(super) fn capture(path: &Path, expected_bytes: u64, expected_sha: &str) -> Result<()> {
    if expected_bytes == 0 || expected_bytes > MAX_CAPTURE_BYTES {
        return Err(too_large().into());
    }
    let (file, length) = open(path, MAX_CAPTURE_BYTES)?;
    if length != expected_bytes {
        bail!("retained NCES capture length differs from its Store receipt");
    }
    let maximum = expected_bytes
        .checked_add(1)
        .context("retained capture limit overflow")?;
    let mut reader = file.take(maximum);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut consumed = 0_u64;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        consumed = consumed
            .checked_add(u64::try_from(count)?)
            .context("retained capture count overflow")?;
        if consumed > expected_bytes {
            return Err(too_large().into());
        }
        hasher.update(buffer.get(..count).ok_or_else(too_large)?);
    }
    if consumed != expected_bytes || format!("{:x}", hasher.finalize()) != expected_sha {
        bail!("retained NCES capture bytes differ from their Store receipt");
    }
    Ok(())
}

fn open(path: &Path, maximum: u64) -> io::Result<(File, u64)> {
    let metadata = std::fs::symlink_metadata(path)?;
    check_file(&metadata, maximum)?;
    let file = File::open(path)?;
    let opened = file.metadata()?;
    check_file(&opened, maximum)?;
    Ok((file, opened.len()))
}

fn check_file(metadata: &std::fs::Metadata, maximum: u64) -> io::Result<()> {
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "retained NCES input is not a regular file",
        ));
    }
    if metadata.len() > maximum {
        return Err(too_large());
    }
    Ok(())
}

fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        "retained NCES input exceeds its byte envelope",
    )
}
