use super::super::archive_io::{create_directory, invalid, io_error, present};
use super::super::FetchError;
use super::inventory::archive_bytes_used;
use super::reservations::{
    read_record, record_path, recover_reservations, remove_record, reservation_bytes, write_record,
};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(in crate::net::cache) const DEFAULT_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const ARCHIVE_CAPACITY_ENV: &str = "CENSUS_ARCHIVE_CAPACITY_BYTES";
const ARCHIVE_LOCK_NAME: &str = "archive-capacity.lock";

pub(in crate::net::cache) struct ArchiveAdmission {
    _lock: File,
    archive: PathBuf,
    capacity: u64,
}

impl ArchiveAdmission {
    pub(in crate::net::cache) fn begin_with_capacity(
        root: &Path,
        archive: &Path,
        capacity: u64,
    ) -> Result<Self, FetchError> {
        let capacity = positive_capacity(capacity, archive)?;
        let lock_path = root.join(ARCHIVE_LOCK_NAME);
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| io_error(&lock_path, error))?;
        lock.lock().map_err(|error| io_error(&lock_path, error))?;
        create_directory(&archive.join("reservations"))?;
        archive_bytes_used(archive)?;
        recover_reservations(archive)?;
        Ok(Self {
            _lock: lock,
            archive: archive.to_path_buf(),
            capacity,
        })
    }

    pub(in crate::net::cache) fn reserve_capture_sizes(
        &mut self,
        content_digest: &str,
        metadata_digest: &str,
        body: Option<u64>,
        metadata: Option<u64>,
    ) -> Result<(), FetchError> {
        let path = record_path(&self.archive, content_digest, metadata_digest)?;
        let bytes = (
            body.map_or(0, |value| value),
            metadata.map_or(0, |value| value),
        );
        if present(&path)? {
            if read_record(&path)? != bytes {
                return Err(invalid(
                    &path,
                    "archive key already has a different reservation",
                ));
            }
            return Ok(());
        }
        self.reserve_bytes(&path, bytes)
    }

    fn reserve_bytes(&self, path: &Path, bytes: (u64, u64)) -> Result<(), FetchError> {
        let additional = bytes
            .0
            .checked_add(bytes.1)
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        if additional == 0 {
            return Ok(());
        }
        self.check_capacity(additional)?;
        match write_record(path, bytes) {
            Ok(()) => Ok(()),
            Err(error) => {
                remove_record(path)?;
                Err(error)
            }
        }
    }

    fn check_capacity(&self, additional: u64) -> Result<(), FetchError> {
        let used = archive_bytes_used(&self.archive)?;
        let reserved = reservation_bytes(&self.archive)?;
        let projected = used
            .checked_add(reserved)
            .and_then(|sum| sum.checked_add(additional))
            .ok_or_else(|| invalid(&self.archive, "archive capacity accounting overflowed"))?;
        if projected > self.capacity {
            return Err(FetchError::ArchiveCapacity {
                archive: self.archive.clone(),
                used,
                requested: additional,
                capacity: self.capacity,
            });
        }
        Ok(())
    }

    pub(in crate::net::cache) fn finish_capture(
        &mut self,
        content_digest: &str,
        metadata_digest: &str,
    ) -> Result<(), FetchError> {
        remove_record(&record_path(
            &self.archive,
            content_digest,
            metadata_digest,
        )?)
    }
}

pub(in crate::net::cache) fn archive_capacity() -> Result<u64, FetchError> {
    match std::env::var(ARCHIVE_CAPACITY_ENV) {
        Ok(value) => capacity_of(Some(value)),
        Err(std::env::VarError::NotPresent) => capacity_of(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(invalid(
            Path::new(ARCHIVE_CAPACITY_ENV),
            "CENSUS_ARCHIVE_CAPACITY_BYTES must contain a Unicode positive u64 byte count",
        )),
    }
}

pub(in crate::net::cache) fn capacity_of(value: Option<String>) -> Result<u64, FetchError> {
    let Some(value) = value else {
        return Ok(DEFAULT_ARCHIVE_BYTES);
    };
    let path = Path::new(ARCHIVE_CAPACITY_ENV);
    let capacity = value.trim().parse::<u64>().map_err(|_| {
        invalid(
            path,
            "CENSUS_ARCHIVE_CAPACITY_BYTES must be a positive u64 byte count",
        )
    })?;
    positive_capacity(capacity, path)
}

fn positive_capacity(capacity: u64, path: &Path) -> Result<u64, FetchError> {
    if capacity == 0 {
        return Err(invalid(
            path,
            "CENSUS_ARCHIVE_CAPACITY_BYTES must be a positive u64 byte count",
        ));
    }
    Ok(capacity)
}

#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;
