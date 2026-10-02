use super::FetchError;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

pub(super) fn io_error(path: &Path, source: io::Error) -> FetchError {
    FetchError::Cache {
        path: path.to_path_buf(),
        source,
    }
}

pub(super) fn invalid(path: &Path, detail: &str) -> FetchError {
    io_error(path, io::Error::new(io::ErrorKind::InvalidData, detail))
}

pub(super) fn present(path: &Path) -> Result<bool, FetchError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(invalid(path, "capture path is not a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(path, error)),
    }
}

pub(super) fn sync_directory(path: &Path) -> Result<(), FetchError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error(path, error))
}

pub(super) fn create_directory(path: &Path) -> Result<(), FetchError> {
    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
            if !metadata.is_dir() {
                return Err(invalid(path, "archive path is not a directory"));
            }
        }
        Err(error) => return Err(io_error(path, error)),
    }
    sync_directory(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid(path, "directory has no parent"))?;
    sync_directory(parent)
}

pub(super) fn lock_cache(meta_path: &Path) -> Result<File, FetchError> {
    let lock_path = meta_path.with_extension("json.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| io_error(&lock_path, error))?;
    file.try_lock().map_err(|error| match error {
        fs::TryLockError::WouldBlock => io_error(
            &lock_path,
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "cache publication is already in progress",
            ),
        ),
        fs::TryLockError::Error(source) => io_error(&lock_path, source),
    })?;
    Ok(file)
}

pub(super) fn with_stage<T>(
    root: &Path,
    action: impl FnOnce(&Path) -> Result<T, FetchError>,
) -> Result<T, FetchError> {
    let stage = new_stage(root)?;
    match action(&stage) {
        Ok(value) => {
            fs::remove_dir_all(&stage).map_err(|error| io_error(&stage, error))?;
            sync_directory(root)?;
            Ok(value)
        }
        Err(error) => {
            sync_directory(&stage)?;
            Err(error)
        }
    }
}

pub(super) fn new_directory(root: &Path, prefix: &str) -> Result<PathBuf, FetchError> {
    let sequence = NEXT_STAGE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| invalid(root, "capture staging sequence exhausted"))?;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io_error(root, io::Error::other(error)))?;
    let path = root.join(format!(
        "{prefix}-{}-{}-{sequence}",
        std::process::id(),
        time.as_nanos()
    ));
    fs::create_dir(&path).map_err(|error| io_error(&path, error))?;
    sync_directory(&path)?;
    sync_directory(root)?;
    Ok(path)
}

fn new_stage(root: &Path) -> Result<PathBuf, FetchError> {
    new_directory(root, ".capture-stage")
}

pub(super) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), FetchError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(path, error))?;
    file.write_all(bytes)
        .map_err(|error| io_error(path, error))?;
    file.sync_all().map_err(|error| io_error(path, error))
}

pub(super) fn freeze_file(path: &Path) -> Result<(), FetchError> {
    let file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut permissions = file
        .metadata()
        .map_err(|error| io_error(path, error))?
        .permissions();
    if !permissions.readonly() {
        permissions.set_readonly(true);
        file.set_permissions(permissions)
            .map_err(|error| io_error(path, error))?;
    }
    file.sync_all().map_err(|error| io_error(path, error))
}

pub(super) fn publish_once(staged: &Path, destination: &Path) -> Result<(), FetchError> {
    match fs::hard_link(staged, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io_error(destination, error)),
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid(destination, "capture has no parent"))?;
    sync_directory(parent)
}

pub(super) fn replace_cache(
    stage: &Path,
    archived_body: &Path,
    body_path: &Path,
    meta_path: &Path,
    encoded: &[u8],
) -> Result<(), FetchError> {
    let body = stage.join("cache.body");
    let meta = stage.join("cache.meta.json");
    fs::hard_link(archived_body, &body).map_err(|error| io_error(&body, error))?;
    write_file(&meta, encoded)?;
    fs::rename(&body, body_path).map_err(|error| io_error(body_path, error))?;
    let parent = body_path
        .parent()
        .ok_or_else(|| invalid(body_path, "cache has no parent"))?;
    sync_directory(parent)?;
    fs::rename(&meta, meta_path).map_err(|error| io_error(meta_path, error))?;
    sync_directory(parent)
}
