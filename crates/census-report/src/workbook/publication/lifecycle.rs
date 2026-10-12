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
    reject_flat_artifacts(root)?;
    validate_layout(root)
}

fn reject_flat_artifacts(root: &Path) -> ReportResult<()> {
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
    Ok(())
}

fn validate_layout(root: &Path) -> ReportResult<()> {
    let path = root.join("generations");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(invariant(format!(
            "invalid publication layout at {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(&path, source)),
    }
}

pub(super) fn finish<T>(result: ReportResult<T>, root: &Path, stage: &Path) -> ReportResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(
            error @ ReportError::Publication(
                super::PublicationError::PublicationDurabilityUncertain { .. },
            ),
        ) => Err(error),
        Err(error) => {
            let cleanup = std::fs::remove_dir_all(stage);
            let cleanup = match cleanup {
                Ok(()) => super::sync_directory(root),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(source) => Err(io_error(stage, source)),
            };
            combine(Err(error), cleanup)
        }
    }
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
