use super::{ArchiveActivityReason, MAX_ACTIVITY_ENTRIES};
use std::path::Path;

pub(super) fn bump(visited: u64) -> Result<u64, ArchiveActivityReason> {
    let next = visited.checked_add(1).ok_or(ArchiveActivityReason::Limit)?;
    if next > MAX_ACTIVITY_ENTRIES {
        return Err(ArchiveActivityReason::Limit);
    }
    Ok(next)
}

pub(super) fn read_archive_dir(dir: &Path) -> Result<std::fs::ReadDir, ArchiveActivityReason> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(ArchiveActivityReason::Missing)
        }
        Err(_) => Err(ArchiveActivityReason::Io),
    }
}

pub(super) fn entry_name(entry: &std::fs::DirEntry) -> Result<String, ArchiveActivityReason> {
    let owned = entry
        .file_name()
        .into_string()
        .map_err(|_| ArchiveActivityReason::UnexpectedLayout)?;
    if owned.is_empty() {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    }
    Ok(owned)
}

pub(super) fn require_dir(path: &Path) -> Result<(), ArchiveActivityReason> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(ArchiveActivityReason::UnexpectedLayout),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(ArchiveActivityReason::Missing)
        }
        Err(_) => Err(ArchiveActivityReason::Io),
    }
}

pub(super) fn regular_file(path: &Path) -> Result<std::fs::Metadata, ArchiveActivityReason> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(metadata),
        Ok(_) => Err(ArchiveActivityReason::UnexpectedLayout),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(ArchiveActivityReason::Corrupt)
        }
        Err(_) => Err(ArchiveActivityReason::Io),
    }
}

pub(super) fn body_length(metadata: &std::fs::Metadata) -> Result<u64, ArchiveActivityReason> {
    let length = metadata.len();
    let ceiling =
        u64::try_from(crate::net::MAX_BODY_BYTES).map_err(|_| ArchiveActivityReason::Limit)?;
    if length > ceiling {
        return Err(ArchiveActivityReason::Limit);
    }
    Ok(length)
}

pub(super) fn body_digest(name: &str) -> Result<&str, ArchiveActivityReason> {
    let Some(digest) = name.strip_suffix(".body") else {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    };
    if !crate::net::cache::is_valid_hex64(digest) {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    }
    Ok(digest)
}

pub(super) fn content_digest_name(name: &str) -> Result<&str, ArchiveActivityReason> {
    if !crate::net::cache::is_valid_hex64(name) {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    }
    Ok(name)
}

pub(super) fn meta_digest(name: &str) -> Result<&str, ArchiveActivityReason> {
    let Some(stem) = name.strip_suffix(".meta.json") else {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    };
    if !crate::net::cache::is_valid_hex64(stem) {
        return Err(ArchiveActivityReason::UnexpectedLayout);
    }
    Ok(stem)
}
