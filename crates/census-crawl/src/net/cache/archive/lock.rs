use super::super::archive_io::io_error;
use super::super::FetchError;
use census_store::{ArchiveWriteFence, StoreError};
use std::path::Path;

#[cfg(test)]
pub(in crate::net::cache) const ARCHIVE_LOCK_NAME: &str = "archive-capacity.lock";

pub(in crate::net) struct ArchiveLock {
    fence: ArchiveWriteFence,
}

impl ArchiveLock {
    pub(in crate::net::cache) fn acquire(root: &Path) -> Result<Self, FetchError> {
        #[cfg(test)]
        let path = root.join(ARCHIVE_LOCK_NAME);
        #[cfg(test)]
        tests::checkpoint(&path, tests::LockEvent::BeforeAcquire)?;
        let fence = ArchiveWriteFence::acquire(root).map_err(fence_error)?;
        #[cfg(test)]
        tests::checkpoint(&path, tests::LockEvent::Acquired)?;
        Ok(Self { fence })
    }

    pub(in crate::net::cache) fn root(&self) -> &Path {
        self.fence.root()
    }
}

fn fence_error(error: StoreError) -> FetchError {
    match error {
        StoreError::Io { path, source } => io_error(&path, source),
        other => other.into(),
    }
}

#[cfg(test)]
pub(in crate::net) mod tests;
