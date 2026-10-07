use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use crate::{StoreError, StoreResult};

#[cfg(unix)]
pub(crate) fn fsync_dir(dir: &Path) -> StoreResult<()> {
    fsync_dir_io(dir).map_err(|source| StoreError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
pub(crate) fn fsync_dir(_dir: &Path) -> StoreResult<()> {
    Ok(())
}

#[cfg(unix)]
fn fsync_dir_io(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn fsync_dir_io(_dir: &Path) -> io::Result<()> {
    Ok(())
}

pub fn create_dir_all_synced(path: &Path) -> io::Result<()> {
    let mut sync = fsync_dir_io;
    create_dir_all_synced_with(path, &mut sync).map(|_| ())
}

pub(crate) fn create_dir_all_synced_with(
    path: &Path,
    sync: &mut impl FnMut(&Path) -> io::Result<()>,
) -> io::Result<u64> {
    let mut probe = |candidate: &Path| match fs::metadata(candidate) {
        Ok(metadata) => Ok(Some(metadata.is_dir())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    };
    let missing = missing_chain(path, &mut probe)?;
    let mut synced = 0_u64;
    for directory in missing.iter().rev() {
        match fs::create_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                if matches!(fs::metadata(directory), Ok(metadata) if metadata.is_dir()) {
                    continue;
                }
                return Err(error);
            }
            Err(error) => return Err(error),
        }
        let parent = match directory.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        sync(parent)?;
        synced = synced.saturating_add(1);
    }
    Ok(synced)
}

pub(crate) fn missing_chain(
    path: &Path,
    probe: &mut impl FnMut(&Path) -> io::Result<Option<bool>>,
) -> io::Result<Vec<PathBuf>> {
    let mut missing: Vec<PathBuf> = Vec::new();
    let mut current = path;
    loop {
        match probe(current)? {
            Some(true) => break,
            Some(false) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} is not a directory", current.display()),
                ));
            }
            None => missing.push(current.to_path_buf()),
        }
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => current = parent,
            _ => break,
        }
    }
    Ok(missing)
}
