use std::path::{Path, PathBuf};

use crate::StoreResult;

use super::io_failure;

pub(super) fn temporary_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "snapshot".to_string());
    path.with_file_name(format!(".{name}.{}.{sequence}.part", std::process::id()))
}

pub(crate) fn sweep_stale_temporaries(root: &Path) -> StoreResult<usize> {
    let mut removed = 0_usize;
    for directory in [root.join("entities"), root.join("out")] {
        removed = removed.saturating_add(sweep_temporaries(&directory)?);
    }
    Ok(removed)
}

fn sweep_temporaries(directory: &Path) -> StoreResult<usize> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(io_failure(directory, source)),
    };
    let mut removed = 0_usize;
    for entry in entries {
        let entry = entry.map_err(|source| io_failure(directory, source))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with('.') || !name.ends_with(".part") {
            continue;
        }
        let path = entry.path();
        std::fs::remove_file(&path).map_err(|source| io_failure(&path, source))?;
        removed = removed.saturating_add(1);
    }
    Ok(removed)
}
