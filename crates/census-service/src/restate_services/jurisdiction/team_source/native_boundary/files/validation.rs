use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

use super::super::error::BoundaryError;
use super::MAX_BYTES;

const MAX_COMPONENTS: usize = 64;
const LINUX_NOFOLLOW: i32 = 0o400000;
const LINUX_NONBLOCK: i32 = 0o4000;

pub(super) fn validate_path(path: &Path) -> Result<(), BoundaryError> {
    if !path.is_absolute() || path.as_os_str().len() > MAX_BYTES {
        return Err(BoundaryError::Path(
            "must be absolute and at most 4096 bytes",
        ));
    }
    if path
        .components()
        .take(MAX_COMPONENTS.saturating_add(1))
        .count()
        > MAX_COMPONENTS
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(BoundaryError::Path(
            "noncanonical or more than 64 components",
        ));
    }
    if path.file_name().is_none() {
        return Err(BoundaryError::Path("missing filename"));
    }
    Ok(())
}

pub(super) fn open_flags() -> Result<i32, BoundaryError> {
    if !cfg!(target_os = "linux") {
        return Err(BoundaryError::Path(
            "native qualification filesystem requires Linux",
        ));
    }
    Ok(LINUX_NOFOLLOW | LINUX_NONBLOCK)
}

pub(super) fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

pub(super) fn unchanged(left: &Metadata, right: &Metadata) -> bool {
    same_file(left, right)
        && left.len() == right.len()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}
