use super::{paths, shell};
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use std::fs::ReadDir;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

struct Frame {
    directory: PathBuf,
    entries: ReadDir,
}

pub(super) fn used(
    root: &Path,
    archive: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<u64, FetchError> {
    let memory = 64_usize
        .checked_mul(std::mem::size_of::<Frame>())
        .and_then(|bytes| {
            4096_usize
                .checked_mul(std::mem::size_of::<u64>())
                .and_then(|inodes| bytes.checked_add(inodes))
        })
        .ok_or_else(|| shell::limit("allocation"))?;
    io.reserve_owned(memory)?;
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(64)
        .map_err(|_| shell::limit("allocation"))?;
    let mut inodes = Vec::new();
    inodes
        .try_reserve_exact(4096)
        .map_err(|_| shell::limit("allocation"))?;
    let reservations = paths::child(archive, "reservations", io)?;
    push(archive, &mut pending, io)?;
    let mut total = 0_u64;
    while let Some(frame) = pending.last_mut() {
        let Some(entry) = shell::next(&mut frame.entries, &frame.directory, io)? else {
            if let Some(frame) = pending.pop() {
                shell::release_directory(&frame.directory, io)?;
                io.release_owned(frame.directory.as_os_str().len())?;
            }
            continue;
        };
        let path = shell::entry_path(entry, &frame.directory, io)?;
        if path != reservations {
            count(&path, &mut pending, &mut inodes, &mut total, io)?;
        }
        io.release_owned(path.as_os_str().len())?;
    }
    count_stages(root, &mut inodes, &mut total, io)?;
    io.release_owned(reservations.as_os_str().len())?;
    io.release_owned(memory)?;
    Ok(total)
}

fn push(
    path: &Path,
    pending: &mut Vec<Frame>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    if pending.len() >= 64 {
        return Err(shell::limit("archive depth"));
    }
    let entries = shell::read_dir(path, io)?;
    shell::sync_directory(path, io)?;
    io.path_bytes(path.as_os_str().len())?;
    io.reserve_owned(path.as_os_str().len())?;
    pending.push(Frame {
        directory: path.to_path_buf(),
        entries,
    });
    Ok(())
}

fn count(
    path: &Path,
    pending: &mut Vec<Frame>,
    inodes: &mut Vec<u64>,
    total: &mut u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let metadata = match shell::kind(path, io)? {
        Some(metadata) => metadata,
        None => return Err(shell::invalid(path, "archive entry changed", io)?),
    };
    if metadata.is_dir() {
        return push(path, pending, io);
    }
    if !metadata.is_file() {
        return Err(shell::invalid(
            path,
            "archive contains an unsafe object",
            io,
        )?);
    }
    count_file(&metadata, inodes, total, io)
}

fn count_file(
    metadata: &std::fs::Metadata,
    inodes: &mut Vec<u64>,
    total: &mut u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    if inodes.contains(&metadata.ino()) {
        return Ok(());
    }
    io.inode_slot()?;
    if inodes.len() >= 4096 {
        return Err(shell::limit("inode slots"));
    }
    inodes.push(metadata.ino());
    *total = total
        .checked_add(metadata.len())
        .ok_or_else(|| shell::limit("archive byte accounting"))?;
    Ok(())
}

fn count_stages(
    root: &Path,
    inodes: &mut Vec<u64>,
    total: &mut u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let mut entries = shell::read_dir(root, io)?;
    while let Some(entry) = shell::next(&mut entries, root, io)? {
        let path = shell::entry_path(entry, root, io)?;
        let stage = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".capture-stage"));
        if stage {
            let metadata = match shell::kind(&path, io)? {
                Some(metadata) => metadata,
                None => return Err(shell::invalid(&path, "stage changed", io)?),
            };
            if !metadata.is_dir() {
                return Err(shell::invalid(
                    &path,
                    "capture stage is not a safe directory",
                    io,
                )?);
            }
            count_stage(&path, inodes, total, io)?;
        }
        io.release_owned(path.as_os_str().len())?;
    }
    shell::release_directory(root, io)
}

fn count_stage(
    path: &Path,
    inodes: &mut Vec<u64>,
    total: &mut u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let mut entries = shell::read_dir(path, io)?;
    while let Some(entry) = shell::next(&mut entries, path, io)? {
        let child = shell::entry_path(entry, path, io)?;
        let metadata = match shell::kind(&child, io)? {
            Some(metadata) => metadata,
            None => return Err(shell::invalid(&child, "stage entry changed", io)?),
        };
        if metadata.is_file() {
            count_file(&metadata, inodes, total, io)?;
        } else {
            return Err(shell::invalid(
                &child,
                "capture stage contains an unsafe object",
                io,
            )?);
        }
        io.release_owned(child.as_os_str().len())?;
    }
    shell::release_directory(path, io)
}
