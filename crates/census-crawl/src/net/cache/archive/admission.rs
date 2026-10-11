use super::super::archive_io::{invalid, io_error};
use super::super::FetchError;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(in crate::net::cache) const DEFAULT_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const ARCHIVE_CAPACITY_ENV: &str = "CENSUS_ARCHIVE_CAPACITY_BYTES";
const ARCHIVE_LOCK_NAME: &str = "archive-capacity.lock";

pub(in crate::net::cache) struct ArchiveAdmission {
    _lock: File,
    archive: PathBuf,
    capacity: u64,
    used: u64,
    reserved: u64,
}

impl ArchiveAdmission {
    pub(in crate::net::cache) fn begin(root: &Path, archive: &Path) -> Result<Self, FetchError> {
        Self::begin_with_capacity(root, archive, archive_capacity())
    }

    pub(in crate::net::cache) fn begin_with_capacity(
        root: &Path,
        archive: &Path,
        capacity: u64,
    ) -> Result<Self, FetchError> {
        let lock_path = root.join(ARCHIVE_LOCK_NAME);
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| io_error(&lock_path, error))?;
        lock.lock().map_err(|error| io_error(&lock_path, error))?;
        let used = archive_bytes_used(archive)?;
        Ok(Self {
            _lock: lock,
            archive: archive.to_path_buf(),
            capacity,
            used,
            reserved: 0,
        })
    }

    pub(super) fn reserve_capture_sizes(
        &mut self,
        body: Option<u64>,
        metadata: Option<u64>,
    ) -> Result<(), FetchError> {
        let additional = body
            .map_or(0, |value| value)
            .checked_add(metadata.map_or(0, |value| value))
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        let reserved = self
            .reserved
            .checked_add(additional)
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        let projected = self
            .used
            .checked_add(reserved)
            .ok_or_else(|| invalid(&self.archive, "archive capacity accounting overflowed"))?;
        if projected > self.capacity {
            return Err(FetchError::ArchiveCapacity {
                archive: self.archive.clone(),
                used: self.used,
                requested: reserved,
                capacity: self.capacity,
            });
        }
        self.reserved = reserved;
        Ok(())
    }
}

fn archive_capacity() -> u64 {
    capacity_of(std::env::var(ARCHIVE_CAPACITY_ENV).ok())
}

pub(in crate::net::cache) fn capacity_of(value: Option<String>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map_or(DEFAULT_ARCHIVE_BYTES, |value| value)
}

fn archive_bytes_used(archive: &Path) -> Result<u64, FetchError> {
    let mut total: u64 = 0;
    let mut pending = vec![archive.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| io_error(&directory, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| io_error(&directory, error))?;
            let path = entry.path();
            let metadata =
                std::fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
            if metadata.file_type().is_symlink() {
                return Err(invalid(
                    &path,
                    "archive entry is a symbolic link, so capacity accounting is refused",
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or_else(|| invalid(&path, "archive byte accounting overflowed"))?;
            } else {
                return Err(invalid(
                    &path,
                    "archive entry is neither a regular file nor a directory",
                ));
            }
        }
    }
    Ok(total)
}
