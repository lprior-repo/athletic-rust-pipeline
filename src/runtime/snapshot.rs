use crate::domain::identity::WorkbookDigest;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

const MAX_WORKBOOK_BYTES: u64 = 1_073_741_824;

pub fn freeze(input: &Path, directory: &Path, expected: &WorkbookDigest) -> Result<PathBuf> {
    let snapshots = directory.join("source-workbooks");
    create_directory(&snapshots)?;
    let target = snapshots.join(format!("{}.xlsx", expected.as_str()));
    if target.try_exists()? {
        verify(&target, expected)?;
        return Ok(target);
    }
    let mut temporary = NamedTempFile::new_in(&snapshots)?;
    copy_verified(input, &mut temporary, expected)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(&target) {
        Ok(_) => File::open(&snapshots)?.sync_all()?,
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
            verify(&target, expected)?
        }
        Err(error) => return Err(error.error).context("publishing immutable workbook snapshot"),
    }
    Ok(target)
}

pub fn verify(path: &Path, expected: &WorkbookDigest) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_WORKBOOK_BYTES {
        bail!("workbook must be a regular file no larger than one GiB");
    }
    let mut sink = HashWriter {
        writer: io::sink(),
        digest: Sha256::new(),
    };
    let bytes = io::copy(
        &mut File::open(path)?.take(MAX_WORKBOOK_BYTES + 1),
        &mut sink,
    )?;
    verify_copied(bytes, sink.digest, expected)
}

fn copy_verified(
    input: &Path,
    output: &mut NamedTempFile,
    expected: &WorkbookDigest,
) -> Result<()> {
    let metadata = fs::symlink_metadata(input)?;
    if !metadata.is_file() || metadata.len() > MAX_WORKBOOK_BYTES {
        bail!("workbook input is not a bounded regular file");
    }
    let mut sink = HashWriter {
        writer: output,
        digest: Sha256::new(),
    };
    let bytes = io::copy(
        &mut File::open(input)?.take(MAX_WORKBOOK_BYTES + 1),
        &mut sink,
    )?;
    verify_copied(bytes, sink.digest, expected)
}

fn verify_copied(bytes: u64, digest: Sha256, expected: &WorkbookDigest) -> Result<()> {
    if bytes > MAX_WORKBOOK_BYTES {
        bail!("workbook exceeds one GiB");
    }
    if format!("{:x}", digest.finalize()) != expected.as_str() {
        bail!("workbook bytes differ from the requested immutable digest");
    }
    Ok(())
}

fn create_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !fs::symlink_metadata(path)?.is_dir() {
                bail!("snapshot directory is not a directory");
            }
            Ok(())
        }
        Err(error) => Err(error).context("creating private workbook snapshot directory"),
    }
}

struct HashWriter<W> {
    writer: W,
    digest: Sha256,
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writer.write_all(bytes)?;
        self.digest.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
