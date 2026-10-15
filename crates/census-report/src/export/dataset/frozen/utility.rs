use super::codec::FrozenCodecError;
use crate::report::{io_error, ExportResourceError, ReportError, ReportResult};
use std::path::{Path, PathBuf};

pub(in crate::export::dataset) fn capture_lock(lock_path: &Path) -> ReportResult<std::fs::File> {
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|source| io_error(lock_path, source))?;
    lock.try_lock().map_err(|error| {
        let source = match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "export input is already being captured",
            ),
            std::fs::TryLockError::Error(source) => source,
        };
        io_error(lock_path, source)
    })?;
    Ok(lock)
}

pub(in crate::export::dataset) fn codec_error(error: FrozenCodecError) -> ReportError {
    match error {
        FrozenCodecError::RowTooLarge { what, bytes, max } => {
            ExportResourceError::RowTooLarge { what, bytes, max }.into()
        }
        FrozenCodecError::BudgetExceeded {
            what,
            needed,
            available,
        } => ExportResourceError::BudgetExceeded {
            what,
            needed,
            available,
        }
        .into(),
        FrozenCodecError::NeedsPartition { what, bytes, max } => {
            ExportResourceError::NeedsPartition { what, bytes, max }.into()
        }
        other => ReportError::ArtifactCorrupt {
            path: PathBuf::new(),
            detail: format!("frozen codec: {other}"),
        },
    }
}
