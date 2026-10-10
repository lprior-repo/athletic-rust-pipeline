use crate::report::{io_error, ReportResult};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static SPILL_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

const SPILL_CREATE_ATTEMPTS: usize = 256;

pub(super) struct SpillDir {
    path: PathBuf,
}

impl SpillDir {
    pub(super) fn create() -> ReportResult<Self> {
        for _ in 0..SPILL_CREATE_ATTEMPTS {
            let sequence = SPILL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "census-performance-rows-{}-{sequence}",
                std::process::id()
            ));
            match try_claim(&path) {
                Ok(true) => return Ok(Self { path }),
                Ok(false) => {}
                Err(source) => return Err(io_error(&path, source)),
            }
        }
        Err(crate::report::ReportError::Invariant {
            detail: "the performance spill cannot claim a fresh directory".to_string(),
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

pub(super) fn try_claim(path: &Path) -> Result<bool, std::io::Error> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => claim_if_empty(path),
        Err(error) => Err(error),
    }
}

pub(super) fn claim_if_empty(path: &Path) -> Result<bool, std::io::Error> {
    if std::fs::remove_dir(path).is_err() {
        return Ok(false);
    }
    match std::fs::create_dir(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

impl Drop for SpillDir {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "could not remove the performance spill directory"
            );
        }
    }
}
