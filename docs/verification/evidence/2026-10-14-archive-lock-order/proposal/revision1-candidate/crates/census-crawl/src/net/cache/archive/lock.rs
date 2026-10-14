use super::super::archive_io::{invalid, io_error};
use super::super::FetchError;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(in crate::net::cache) const ARCHIVE_LOCK_NAME: &str = "archive-capacity.lock";

pub(in crate::net) struct ArchiveLock {
    _file: File,
    root: PathBuf,
}

impl ArchiveLock {
    pub(in crate::net::cache) fn acquire(root: &Path) -> Result<Self, FetchError> {
        let metadata = std::fs::symlink_metadata(root).map_err(|error| io_error(root, error))?;
        if !metadata.is_dir() {
            return Err(invalid(root, "archive root is not a directory"));
        }
        let path = root.join(ARCHIVE_LOCK_NAME);
        #[cfg(test)]
        tests::checkpoint(&path, tests::LockEvent::BeforeAcquire)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| io_error(&path, error))?;
        file.lock().map_err(|error| io_error(&path, error))?;
        #[cfg(test)]
        tests::checkpoint(&path, tests::LockEvent::Acquired)?;
        Ok(Self {
            _file: file,
            root: root.to_path_buf(),
        })
    }

    pub(in crate::net::cache) fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
pub(in crate::net) mod tests;
