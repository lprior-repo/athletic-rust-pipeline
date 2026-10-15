use std::path::Path;

#[cfg(unix)]
mod scan;
#[cfg(all(test, unix))]
mod tests;
#[cfg(unix)]
mod validate;

pub const ARCHIVE_ACTIVITY_METRIC_VERSION: u32 = 1;
#[cfg(unix)]
pub const MAX_ACTIVITY_ENTRIES: u64 = 262144;
#[cfg(unix)]
pub const MAX_ACTIVITY_META_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveActivityReason {
    FenceBusy,
    Missing,
    LockIo,
    Io,
    UnexpectedLayout,
    Corrupt,
    Limit,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveActivitySample {
    pub capture_documents: u64,
    pub unique_body_files: u64,
    pub unique_body_payload_bytes: u64,
    pub archive_dev: u64,
    pub archive_ino: u64,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveActivityScan {
    Measured(ArchiveActivitySample),
    Unmeasured { reason: ArchiveActivityReason },
}

#[cfg(unix)]
pub fn sample_archive_artifacts(
    http_root: &Path,
    started_at: &str,
    clock: &dyn census_store::clock::Clock,
) -> ArchiveActivityScan {
    let fence = match acquire(http_root) {
        Ok(fence) => fence,
        Err(reason) => return ArchiveActivityScan::Unmeasured { reason },
    };
    let archive = fence.root().join("archive");
    let (dev, ino) = match archive_ids(&archive) {
        Ok(ids) => ids,
        Err(reason) => return ArchiveActivityScan::Unmeasured { reason },
    };
    let counts = match scan::count_addresses(&archive) {
        Ok(counts) => counts,
        Err(reason) => return ArchiveActivityScan::Unmeasured { reason },
    };
    let finished = clock.today_iso8601();
    let _held = fence;
    ArchiveActivityScan::Measured(ArchiveActivitySample {
        capture_documents: counts.0,
        unique_body_files: counts.1,
        unique_body_payload_bytes: counts.2,
        archive_dev: dev,
        archive_ino: ino,
        started_at: started_at.to_string(),
        finished_at: finished,
    })
}

#[cfg(unix)]
fn acquire(http_root: &Path) -> Result<census_store::ArchiveWriteFence, ArchiveActivityReason> {
    match census_store::ArchiveWriteFence::try_acquire_existing(http_root) {
        Ok(Some(fence)) => Ok(fence),
        Ok(None) => Err(ArchiveActivityReason::FenceBusy),
        Err(census_store::StoreError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Err(ArchiveActivityReason::Missing)
        }
        Err(census_store::StoreError::Capture(_)) => Err(ArchiveActivityReason::UnexpectedLayout),
        Err(_) => Err(ArchiveActivityReason::LockIo),
    }
}

#[cfg(unix)]
fn archive_ids(archive: &Path) -> Result<(u64, u64), ArchiveActivityReason> {
    use std::os::unix::fs::MetadataExt;
    let metadata = match std::fs::symlink_metadata(archive) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(ArchiveActivityReason::Missing)
        }
        Err(_) => return Err(ArchiveActivityReason::Io),
    };
    if !metadata.is_dir() {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    }
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
pub fn sample_archive_artifacts(
    _http_root: &Path,
    _started_at: &str,
    _clock: &dyn census_store::clock::Clock,
) -> ArchiveActivityScan {
    ArchiveActivityScan::Unmeasured {
        reason: ArchiveActivityReason::Unsupported,
    }
}
