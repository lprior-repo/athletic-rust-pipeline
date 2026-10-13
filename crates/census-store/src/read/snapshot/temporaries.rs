use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{StoreError, StoreResult};

use super::io_failure;

const MAX_SWEEP_KEYS: usize = 4096;
const MAX_SWEEP_TIME: Duration = Duration::from_millis(10);
const MAX_SWEEP_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn temporary_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = match path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
    {
        Some(value) => value,
        None => "snapshot".to_string(),
    };
    path.with_file_name(format!(".{name}.{}.{sequence}.part", std::process::id()))
}

pub(crate) fn sweep_stale_temporaries(root: &Path, meta: &fjall::Keyspace) -> StoreResult<usize> {
    for prefix in [
        b"records/".as_slice(),
        b"publication/".as_slice(),
        b"lease/".as_slice(),
    ] {
        if let Some(guard) = meta.prefix(prefix).next() {
            guard.key().map_err(|source| StoreError::Read { source })?;
            return Ok(0);
        }
    }
    let started = Instant::now();
    let mut bytes = 0_u64;
    let mut examined = 0_usize;
    [root.join("entities"), root.join("out")]
        .iter()
        .try_fold(0_usize, |removed, directory| {
            removed
                .checked_add(sweep_temporaries(
                    directory,
                    started,
                    &mut bytes,
                    &mut examined,
                )?)
                .ok_or(StoreError::CounterOverflow)
        })
}

fn sweep_temporaries(
    directory: &Path,
    started: Instant,
    bytes: &mut u64,
    examined: &mut usize,
) -> StoreResult<usize> {
    let metadata = match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() => metadata,
        Ok(_) => return Ok(0),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(io_failure(directory, source)),
    };
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    let entries = std::fs::read_dir(directory).map_err(|source| io_failure(directory, source))?;
    let mut removed = 0_usize;
    for entry in entries {
        if *examined >= MAX_SWEEP_KEYS
            || started.elapsed() >= MAX_SWEEP_TIME
            || *bytes >= MAX_SWEEP_BYTES
        {
            break;
        }
        *examined = examined.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        let entry = entry.map_err(|source| io_failure(directory, source))?;
        if !owner_dead(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|source| io_failure(&path, source))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let size = metadata.len();
        match std::fs::remove_file(&path) {
            Ok(()) => {
                *bytes = bytes.checked_add(size).ok_or(StoreError::CounterOverflow)?;
                removed = removed.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(io_failure(&path, source)),
        }
    }
    Ok(removed)
}

fn owner_dead(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".part") else {
        return false;
    };
    let mut parts = stem.rsplitn(3, '.');
    let (Some(sequence), Some(pid), Some(target)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if sequence.parse::<u64>().is_err() || !target.starts_with('.') || target.len() < 2 {
        return false;
    }
    let Ok(pid) = pid.parse::<u32>() else {
        return false;
    };
    if pid == 0 || !Path::new("/proc/self").is_dir() {
        return false;
    }
    matches!(std::fs::symlink_metadata(format!("/proc/{pid}")), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
}
