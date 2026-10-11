use super::super::archive_io::{invalid, io_error};
use super::super::FetchError;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::net::cache) const DEFAULT_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
const ARCHIVE_CAPACITY_ENV: &str = "CENSUS_ARCHIVE_CAPACITY_BYTES";
const ARCHIVE_LOCK_NAME: &str = "archive-capacity.lock";
const RESERVATIONS_DIR: &str = "reservations";

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
        let reserved = reservation_bytes(archive)?;
        initialize_reservations(archive)?;
        recover_stale_reservations(archive)?;
        Ok(Self {
            _lock: lock,
            archive: archive.to_path_buf(),
            capacity,
            used,
            reserved,
        })
    }

    pub(super) fn reserve_capture_sizes(
        &mut self,
        content_digest: &str,
        metadata_digest: &str,
        body: Option<u64>,
        metadata: Option<u64>,
    ) -> Result<(), FetchError> {
        let additional = body
            .map_or(0, |value| value)
            .checked_add(metadata.map_or(0, |value| value))
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        let current_used = archive_bytes_used(&self.archive)?;
        let current_reserved = reservation_bytes(&self.archive)?;
        let projected = current_used
            .checked_add(current_reserved)
            .ok_or_else(|| invalid(&self.archive, "archive capacity accounting overflowed"))?
            .checked_add(additional)
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        if projected > self.capacity {
            return Err(FetchError::ArchiveCapacity {
                archive: self.archive.clone(),
                used: current_used,
                requested: additional,
                capacity: self.capacity,
            });
        }
        write_reservation(
            &self.archive,
            content_digest,
            metadata_digest,
            body.unwrap_or(0),
            metadata.unwrap_or(0),
        )?;
        self.used = current_used;
        self.reserved = current_reserved
            .checked_add(additional)
            .ok_or_else(|| invalid(&self.archive, "archive reservation overflowed"))?;
        Ok(())
    }

    pub(super) fn settle_capture_sizes(
        &mut self,
        content_digest: &str,
        metadata_digest: &str,
        body: Option<u64>,
        metadata: Option<u64>,
    ) -> Result<(), FetchError> {
        let released = body
            .map_or(0, |value| value)
            .checked_add(metadata.map_or(0, |value| value))
            .ok_or_else(|| invalid(&self.archive, "archive settlement overflowed"))?;
        self.reserved = self
            .reserved
            .checked_sub(released)
            .ok_or_else(|| invalid(&self.archive, "archive settlement exceeds reservation"))?;
        self.used = self
            .used
            .checked_add(released)
            .ok_or_else(|| invalid(&self.archive, "archive settlement overflowed"))?;
        remove_reservation(&self.archive, content_digest, metadata_digest)?;
        Ok(())
    }

    pub(super) fn release_capture_sizes(
        &mut self,
        content_digest: &str,
        metadata_digest: &str,
        body: Option<u64>,
        metadata: Option<u64>,
    ) -> Result<(), FetchError> {
        let released = body
            .map_or(0, |value| value)
            .checked_add(metadata.map_or(0, |value| value))
            .ok_or_else(|| invalid(&self.archive, "archive release overflowed"))?;
        self.reserved = self
            .reserved
            .checked_sub(released)
            .ok_or_else(|| invalid(&self.archive, "archive release exceeds reservation"))?;
        remove_reservation(&self.archive, content_digest, metadata_digest)?;
        Ok(())
    }
}

fn reservation_path(
    archive: &Path,
    content_digest: &str,
    metadata_digest: &str,
) -> PathBuf {
    archive.join(RESERVATIONS_DIR).join(format!("{content_digest}-{metadata_digest}"))
}

fn initialize_reservations(archive: &Path) -> Result<(), FetchError> {
    let dir = archive.join(RESERVATIONS_DIR);
    fs::create_dir_all(&dir).map_err(|error| io_error(&dir, error))?;
    Ok(())
}

fn write_reservation(
    archive: &Path,
    content_digest: &str,
    metadata_digest: &str,
    body_bytes: u64,
    metadata_bytes: u64,
) -> Result<(), FetchError> {
    let path = reservation_path(archive, content_digest, metadata_digest);
    let record = format!("{body_bytes}\n{metadata_bytes}");
    fs::write(&path, record).map_err(|error| io_error(&path, error))
}

fn remove_reservation(
    archive: &Path,
    content_digest: &str,
    metadata_digest: &str,
) -> Result<(), FetchError> {
    let path = reservation_path(archive, content_digest, metadata_digest);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| io_error(&path, error))?;
    }
    Ok(())
}

fn reservation_bytes(archive: &Path) -> Result<u64, FetchError> {
    let dir = archive.join(RESERVATIONS_DIR);
    if !dir.exists() {
        return Ok(0);
    }
    let mut total: u64 = 0;
    for entry in fs::read_dir(&dir).map_err(|error| io_error(&dir, error))? {
        let entry = entry.map_err(|error| io_error(&dir, error))?;
        let path = entry.path();
        if path.is_file() {
            let content = fs::read_to_string(&path).map_err(|error| io_error(&path, error))?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let bytes = trimmed.parse::<u64>().map_err(|_| {
                    invalid(&path, "reservation file contains a non-numeric line")
                })?;
                total = total
                    .checked_add(bytes)
                    .ok_or_else(|| invalid(&path, "reservation byte accounting overflowed"))?;
            }
        }
    }
    Ok(total)
}
fn recover_stale_reservations(archive: &Path) -> Result<(), FetchError> {
    let dir = archive.join(RESERVATIONS_DIR);
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&dir).map_err(|error| io_error(&dir, error))? {
        let entry = entry.map_err(|error| io_error(&dir, error))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = path
            .file_name()
            .ok_or_else(|| invalid(&path, "reservation file has no name"))?
            .to_string_lossy();
        let parts: Vec<&str> = filename.splitn(2, "-").collect();
        if parts.len() != 2 {
            continue;
        }
        let content_digest = parts[0];
        let metadata_digest = parts[1];
        let captured_body = archive
            .join("bodies")
            .join(format!("{content_digest}.body"));
        let captured_meta = archive
            .join("captures")
            .join(content_digest)
            .join(format!("{metadata_digest}.meta.json"));
        if captured_body.exists() && captured_meta.exists() {
            remove_reservation(archive, content_digest, metadata_digest)?;
        }
    }
    Ok(())
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
        if directory == archive.join(RESERVATIONS_DIR) {
            continue;
        }
        let entries = fs::read_dir(&directory).map_err(|error| io_error(&directory, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| io_error(&directory, error))?;
            let path = entry.path();
            if path == archive.join(RESERVATIONS_DIR) {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
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
