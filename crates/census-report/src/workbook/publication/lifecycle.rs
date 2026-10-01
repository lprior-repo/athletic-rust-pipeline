use super::{invariant, io_error, ReportError, ReportResult};
use std::path::Path;

pub(super) fn preflight(root: &Path) -> ReportResult<()> {
    match std::fs::symlink_metadata(root) {
        Ok(metadata) if !metadata.file_type().is_dir() => {
            return Err(invariant(
                "publication root must be a real directory".into(),
            ))
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error(root, source)),
    }
    for name in [
        "workbook.xlsx",
        "frozen-input.json",
        "manifest.json",
        "recruiting.csv",
        "audit.json",
    ] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => return Err(invariant(format!("legacy flat artifact {} cannot be a publication root; choose a new root and consume current/workbook.xlsx", path.display()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(io_error(&path, source)),
        }
    }
    for (name, directory) in [("generations", true), ("current", false)] {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata)
                if if directory {
                    metadata.file_type().is_dir()
                } else {
                    metadata.file_type().is_symlink()
                } => {}
            Ok(_) => {
                return Err(invariant(format!(
                    "invalid publication layout at {}",
                    path.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(io_error(&path, source)),
        }
    }
    Ok(())
}

pub(super) fn sweep(root: &Path, identity: &str) -> ReportResult<()> {
    let entries = std::fs::read_dir(root).map_err(|source| io_error(root, source))?;
    let stage_prefix = format!(".staging.{identity}.");
    let pointer_prefix = format!(".current.{identity}.");
    for (index, entry) in entries.enumerate() {
        if index >= 8192 {
            return Err(invariant(
                "publication root exceeds the 8192-entry maintenance budget".into(),
            ));
        }
        let entry = entry.map_err(|source| io_error(root, source))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let staging = owned(name, &stage_prefix, "");
        let pointer = owned(name, &pointer_prefix, ".tmp");
        if staging || pointer {
            remove(&entry.path(), staging)?;
        }
    }
    super::sync_directory(root)
}

fn owned(name: &str, prefix: &str, suffix: &str) -> bool {
    let Some(rest) = name
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
    else {
        return false;
    };
    let Some((pid, time)) = rest.split_once('.') else {
        return false;
    };
    pid.parse::<u32>().is_ok() && time.parse::<u128>().is_ok()
}

pub(super) fn finish<T>(
    result: ReportResult<T>,
    root: &Path,
    stage: &Path,
    pointer: &Path,
) -> ReportResult<T> {
    let stage_cleanup = remove(stage, true);
    let pointer_cleanup = remove(pointer, false);
    let cleanup =
        combine(stage_cleanup, pointer_cleanup).and_then(|()| super::sync_directory(root));
    combine(result, cleanup)
}

fn combine<T>(result: ReportResult<T>, cleanup: ReportResult<()>) -> ReportResult<T> {
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(operation), Err(cleanup)) => Err(ReportError::Cleanup {
            operation: Box::new(operation),
            cleanup: Box::new(cleanup),
        }),
    }
}

fn remove(path: &Path, directory: bool) -> ReportResult<()> {
    let result = if directory {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(path, source)),
    }
}
