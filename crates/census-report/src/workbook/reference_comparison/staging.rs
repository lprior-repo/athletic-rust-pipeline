use super::error;
use crate::report::{io_error, ReportResult};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_BYTES: u64 = 144 * 1024 * 1024;
const MAX_FILES: usize = 2;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Staging {
    directory: PathBuf,
    bytes: u64,
    files: usize,
}

impl Staging {
    pub(super) fn run<T>(action: impl FnOnce(&mut Self) -> ReportResult<T>) -> ReportResult<T> {
        let mut stage = Self::create()?;
        let outcome = action(&mut stage);
        let cleanup = std::fs::remove_dir_all(&stage.directory)
            .map_err(|source| io_error(&stage.directory, source));
        match (outcome, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(cause), Ok(())) | (Ok(_), Err(cause)) => Err(cause),
            (Err(cause), Err(cleanup)) => Err(error(format!(
                "comparison failed: {cause}; staging cleanup failed: {cleanup}"
            ))),
        }
    }

    fn create() -> ReportResult<Self> {
        let root = std::env::temp_dir();
        for _ in 0..128 {
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let directory = root.join(format!("census-reference-{}-{number}", std::process::id()));
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&directory) {
                Ok(()) => {
                    return Ok(Self {
                        directory,
                        bytes: 0,
                        files: 0,
                    })
                }
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(io_error(&directory, source)),
            }
        }
        Err(error("cannot allocate private comparison staging"))
    }

    pub(super) fn copy(
        &mut self,
        source: &Path,
        bytes: u64,
        digest: &str,
    ) -> ReportResult<PathBuf> {
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| error("staging byte budget overflow"))?;
        if total > MAX_BYTES || self.files >= MAX_FILES {
            return Err(error("comparison staging budget exhausted"));
        }
        let target = self.directory.join(self.files.to_string());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options
            .open(&target)
            .map_err(|cause| io_error(&target, cause))?;
        copy_verified(source, &mut output, bytes, digest)?;
        self.bytes = total;
        self.files += 1;
        Ok(target)
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(cause) = std::fs::remove_dir_all(&self.directory) {
            if cause.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(error = %cause, "comparison staging cleanup failed");
            }
        }
    }
}

fn copy_verified(path: &Path, output: &mut File, expected: u64, digest: &str) -> ReportResult<()> {
    let mut input = File::open(path).map_err(|cause| io_error(path, cause))?;
    let metadata = input.metadata().map_err(|cause| io_error(path, cause))?;
    if !metadata.is_file() || metadata.len() != expected {
        return Err(error(
            "comparison input length or file kind differs from its binding",
        ));
    }
    let mut hash = Sha256::new();
    let mut remaining = expected;
    let mut buffer = [0_u8; 65_536];
    while remaining > 0 {
        let allowed =
            usize::try_from(remaining.min(65_536)).map_err(|_| error("copy length overflow"))?;
        let slot = buffer
            .get_mut(..allowed)
            .ok_or_else(|| error("copy buffer bounds"))?;
        let read = input.read(slot).map_err(|cause| io_error(path, cause))?;
        if read == 0 {
            return Err(error("comparison input ended before its bound"));
        }
        let chunk = slot.get(..read).ok_or_else(|| error("copy read bounds"))?;
        hash.update(chunk);
        output
            .write_all(chunk)
            .map_err(|cause| io_error(path, cause))?;
        remaining -= u64::try_from(read).map_err(|_| error("copy counter overflow"))?;
    }
    let mut extra = [0_u8; 1];
    if input
        .read(&mut extra)
        .map_err(|cause| io_error(path, cause))?
        != 0
        || format!("{:x}", hash.finalize()) != digest
    {
        return Err(error(
            "comparison input bytes differ from their digest binding",
        ));
    }
    output.flush().map_err(|cause| io_error(path, cause))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_staging_binds_bytes_and_cleans_up_all_outcomes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("source");
        std::fs::write(&source, b"example")?;
        let digest = format!("{:x}", Sha256::digest(b"example"));
        let staged = Staging::run(|stage| {
            let path = stage.copy(&source, 7, &digest)?;
            std::fs::write(&source, b"changed").map_err(|cause| io_error(&source, cause))?;
            let bytes = std::fs::read(&path).map_err(|cause| io_error(&path, cause))?;
            assert_eq!(bytes, b"example");
            Ok(path)
        })?;
        assert!(!staged.exists());
        let mut failed_directory = PathBuf::new();
        let outcome = Staging::run(|stage| {
            failed_directory = stage.directory.clone();
            stage.copy(&source, 7, &digest)
        });
        assert!(outcome.is_err());
        assert!(!failed_directory.exists());
        Ok(())
    }

    #[test]
    fn aggregate_disk_and_entry_limits_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("source");
        std::fs::write(&source, b"x")?;
        let digest = format!("{:x}", Sha256::digest(b"x"));
        Staging::run(|stage| {
            assert!(stage.copy(&source, MAX_BYTES + 1, &digest).is_err());
            stage.copy(&source, 1, &digest)?;
            stage.copy(&source, 1, &digest)?;
            assert!(stage.copy(&source, 1, &digest).is_err());
            Ok(())
        })?;
        Ok(())
    }
}
