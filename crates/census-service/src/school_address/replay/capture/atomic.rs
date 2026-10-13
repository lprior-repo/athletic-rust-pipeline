use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{identical, io, refused, ReplayError, MAX_CAPTURE_BYTES};

const TEMP_ATTEMPTS: u32 = 32;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) enum InstallOutcome {
    Retained,
    Occupied,
}

pub(super) fn with_staged<T>(
    path: &Path,
    bytes: &[u8],
    install: impl FnOnce(&Path) -> Result<T, ReplayError>,
) -> Result<T, ReplayError> {
    let size = u64::try_from(bytes.len())
        .map_err(|_| refused(path, "capture size is not representable"))?;
    if size > MAX_CAPTURE_BYTES {
        return Err(refused(path, "atomic write exceeds its byte limit"));
    }
    with_temp(path, |tmp, file| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| io(tmp, source))?;
        if !identical(tmp, bytes)? {
            return Err(refused(tmp, "atomic write verification failed"));
        }
        install(tmp)
    })
}

pub(super) fn with_temp<T>(
    path: &Path,
    operation: impl FnOnce(&Path, &mut File) -> Result<T, ReplayError>,
) -> Result<T, ReplayError> {
    let (tmp, mut file) = create_temp(path)?;
    let result = operation(&tmp, &mut file);
    drop(file);
    finish_cleanup(&tmp, result)
}

fn create_temp(path: &Path) -> Result<(PathBuf, File), ReplayError> {
    (0..TEMP_ATTEMPTS)
        .find_map(|_| match create_one(path) {
            Ok(Some(created)) => Some(Ok(created)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .ok_or_else(|| refused(path, "atomic temporary destinations are occupied"))?
}

fn create_one(path: &Path) -> Result<Option<(PathBuf, File)>, ReplayError> {
    let counter = TEMP_COUNTER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| refused(path, "atomic temporary counter is exhausted"))?;
    let tmp = path.with_extension(format!("tmp.{}.{}", std::process::id(), counter));
    match OpenOptions::new().write(true).create_new(true).open(&tmp) {
        Ok(file) => Ok(Some((tmp, file))),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
        Err(source) => Err(io(&tmp, source)),
    }
}

fn finish_cleanup<T>(path: &Path, result: Result<T, ReplayError>) -> Result<T, ReplayError> {
    match std::fs::remove_file(path) {
        Ok(()) => result,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => result,
        Err(source) => match result {
            Ok(_) => Err(io(path, source)),
            Err(primary) => Err(ReplayError::Cleanup {
                path: path.to_path_buf(),
                primary: Box::new(primary),
                source,
            }),
        },
    }
}

pub(super) fn install_new<S>(
    path: &Path,
    bytes: &[u8],
    sync: &S,
) -> Result<InstallOutcome, ReplayError>
where
    S: Fn(&Path) -> Result<(), ReplayError>,
{
    with_staged(path, bytes, |tmp| match std::fs::hard_link(tmp, path) {
        Ok(()) => Ok(InstallOutcome::Retained),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(path, bytes)? {
                sync(path)?;
                Ok(InstallOutcome::Retained)
            } else {
                Ok(InstallOutcome::Occupied)
            }
        }
        Err(source) => Err(io(path, source)),
    })
}

pub(super) fn sync_regular(path: &Path) -> Result<(), ReplayError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.is_file() {
        return Err(refused(path, "retained data is not a regular file"));
    }
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io(path, source))
}

pub(super) fn check_destination(path: &Path) -> Result<(), ReplayError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(refused(
            path,
            "publication destination is not a regular file",
        )),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io(path, source)),
    }
}

pub(super) fn fsync_parent(path: &Path) -> Result<(), ReplayError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or(Path::new("."), core::convert::identity);
    File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| io(parent, source))
}
