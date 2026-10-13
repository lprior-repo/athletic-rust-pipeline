use super::super::archive_io::{invalid, io_error, sync_directory};
use super::super::FetchError;
use std::collections::HashSet;
use std::fs::{self, ReadDir};
use std::path::{Path, PathBuf};

const MAX_ARCHIVE_DEPTH: usize = 64;

type Directories = Vec<(PathBuf, ReadDir)>;

pub(super) fn archive_bytes_used(root: &Path, archive: &Path) -> Result<u64, FetchError> {
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(MAX_ARCHIVE_DEPTH)
        .map_err(|_| invalid(archive, "archive inventory allocation failed"))?;
    open_directory(archive, &mut pending)?;
    let reservations = archive.join("reservations");
    let mut counted = HashSet::new();
    let mut total = 0_u64;
    while let Some((directory, entries)) = pending.last_mut() {
        let Some(entry) = entries.next() else {
            drop(pending.pop());
            continue;
        };
        let entry = entry.map_err(|error| io_error(directory, error))?;
        let path = entry.path();
        if path != reservations {
            inventory_entry(&path, &mut pending, &mut counted, &mut total)?;
        }
    }
    count_stages(root, &mut counted, &mut total)?;
    Ok(total)
}

fn open_directory(path: &Path, pending: &mut Directories) -> Result<(), FetchError> {
    if pending.len() >= MAX_ARCHIVE_DEPTH {
        return Err(invalid(path, "archive exceeds the inventory depth limit"));
    }
    let entries = fs::read_dir(path).map_err(|error| io_error(path, error))?;
    sync_directory(path)?;
    pending.push((path.to_path_buf(), entries));
    Ok(())
}

fn inventory_entry(
    path: &Path,
    pending: &mut Directories,
    counted: &mut HashSet<u64>,
    total: &mut u64,
) -> Result<(), FetchError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(invalid(
            path,
            "archive entry is a symbolic link, so capacity accounting is refused",
        ));
    }
    if metadata.is_dir() {
        open_directory(path, pending)
    } else if metadata.is_file() {
        let inode = std::os::unix::fs::MetadataExt::ino(&metadata);
        if !counted.insert(inode) {
            return Ok(());
        }
        *total = total
            .checked_add(metadata.len())
            .ok_or_else(|| invalid(path, "archive byte accounting overflowed"))?;
        Ok(())
    } else {
        Err(invalid(
            path,
            "archive entry is neither a regular file nor a directory",
        ))
    }
}

fn count_stages(
    root: &Path,
    counted: &mut HashSet<u64>,
    total: &mut u64,
) -> Result<(), FetchError> {
    let entries = fs::read_dir(root).map_err(|error| io_error(root, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(root, error))?;
        let path = entry.path();
        let is_stage = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".capture-stage"));
        if !is_stage {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
        if !metadata.is_dir() {
            continue;
        }
        count_stage_contents(&path, counted, total)?;
    }
    Ok(())
}

fn count_stage_contents(
    path: &Path,
    counted: &mut HashSet<u64>,
    total: &mut u64,
) -> Result<(), FetchError> {
    let entries = fs::read_dir(path).map_err(|error| io_error(path, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(path, error))?;
        let file_path = entry.path();
        let metadata =
            fs::symlink_metadata(&file_path).map_err(|error| io_error(&file_path, error))?;
        if metadata.is_file() {
            let inode = std::os::unix::fs::MetadataExt::ino(&metadata);
            if !counted.insert(inode) {
                continue;
            }
            *total = total
                .checked_add(metadata.len())
                .ok_or_else(|| invalid(&file_path, "archive byte accounting overflowed"))?;
        }
    }
    Ok(())
}
