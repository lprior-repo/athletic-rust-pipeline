use super::shell;
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use std::fmt::{self, Display, Write};
use std::path::Path;

pub(super) fn admit_error_boxes(io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    let bytes = std::mem::size_of::<census_store::StoreError>()
        .checked_mul(5)
        .ok_or_else(|| shell::limit("allocation"))?;
    Ok(io.reserve_owned(bytes)?)
}

struct Count {
    bytes: usize,
    exceeded: bool,
}

impl Write for Count {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let Some(bytes) = self.bytes.checked_add(value.len()) else {
            self.exceeded = true;
            return Err(fmt::Error);
        };
        if bytes > 8192 {
            self.exceeded = true;
            return Err(fmt::Error);
        }
        self.bytes = bytes;
        Ok(())
    }
}

pub(super) fn error(
    source: &impl Display,
    owned: usize,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let mut count = Count {
        bytes: owned,
        exceeded: owned > 8192,
    };
    let result = write!(&mut count, "{source}");
    if count.exceeded {
        return Ok(io.diagnostic_bytes(8193)?);
    }
    if result.is_err() {
        return Err(shell::limit("diagnostic bytes"));
    }
    Ok(io.diagnostic_bytes(count.bytes)?)
}

#[derive(Debug)]
pub(super) struct CacheCleanupFailure {
    pub(super) original: std::io::Error,
    pub(super) cleanup: [Option<crate::net::RecoveryCleanupLeaf>; 2],
}

impl Display for CacheCleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.original, formatter)?;
        for error in self.cleanup.iter().flatten() {
            write!(formatter, "; cleanup error: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CacheCleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.original)
    }
}

pub(super) fn with_cleanup<T>(
    _path: &Path,
    original: Result<T, FetchError>,
    cleanup: Result<(), FetchError>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<T, FetchError> {
    match (original, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(error), Ok(())) => Err(error),
        (Err(original), Err(cleanup)) => Err(combine(original, cleanup, io)?),
    }
}

fn leaf(error: FetchError) -> Result<crate::net::RecoveryCleanupLeaf, FetchError> {
    match error {
        FetchError::CaptureStore(error) => Ok(crate::net::RecoveryCleanupLeaf::Store(error)),
        FetchError::Cache { path, source } => {
            Ok(crate::net::RecoveryCleanupLeaf::Cache { path, source })
        }
        _ => Err(census_store::CaptureError::TransitionRefused.into()),
    }
}

fn push(
    slots: &mut [Option<crate::net::RecoveryCleanupLeaf>; 2],
    value: crate::net::RecoveryCleanupLeaf,
) -> Result<(), FetchError> {
    let slot = slots
        .iter_mut()
        .find(|slot| slot.is_none())
        .ok_or(census_store::CaptureError::TransitionRefused)?;
    *slot = Some(value);
    Ok(())
}

fn reserve(bytes: usize, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    let bytes = bytes
        .checked_add(512)
        .ok_or_else(|| shell::limit("diagnostic bytes"))?;
    io.diagnostic_bytes(bytes)?;
    Ok(io.reserve_owned(bytes)?)
}

fn combine(
    original: FetchError,
    cleanup: FetchError,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<FetchError, FetchError> {
    let cleanup = leaf(cleanup)?;
    if matches!(cleanup, crate::net::RecoveryCleanupLeaf::Store(_)) {
        io.diagnostic_bytes(512)?;
    }
    match original {
        FetchError::Cache { path, source } => combine_cache(path, source, cleanup, io),
        FetchError::RecoveryCleanup(error) => {
            let crate::net::RecoveryCleanupFailure {
                primary,
                cleanup: mut slots,
            } = *error;
            push(&mut slots, cleanup)?;
            reserve(
                std::mem::size_of::<crate::net::RecoveryCleanupFailure>(),
                io,
            )?;
            Ok(FetchError::RecoveryCleanup(Box::new(
                crate::net::RecoveryCleanupFailure::from_parts(primary, slots),
            )))
        }
        original => {
            let primary = match original {
                FetchError::CaptureStore(error) => crate::net::RecoveryFailurePrimary::Store(error),
                FetchError::Capture(error) => crate::net::RecoveryFailurePrimary::Capture(error),
                FetchError::Encode { target, source } => {
                    crate::net::RecoveryFailurePrimary::Encode { target, source }
                }
                _ => return Err(census_store::CaptureError::TransitionRefused.into()),
            };
            reserve(
                std::mem::size_of::<crate::net::RecoveryCleanupFailure>(),
                io,
            )?;
            Ok(FetchError::RecoveryCleanup(Box::new(
                crate::net::RecoveryCleanupFailure::from_parts(primary, [Some(cleanup), None]),
            )))
        }
    }
}

fn combine_cache(
    path: std::path::PathBuf,
    source: std::io::Error,
    cleanup: crate::net::RecoveryCleanupLeaf,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<FetchError, FetchError> {
    let kind = source.kind();
    let (original, mut slots) = if source
        .get_ref()
        .is_some_and(|error| error.is::<CacheCleanupFailure>())
    {
        let source = source
            .into_inner()
            .ok_or(census_store::CaptureError::TransitionRefused)?;
        let held = source
            .downcast::<CacheCleanupFailure>()
            .map_err(|_| census_store::CaptureError::TransitionRefused)?;
        (held.original, held.cleanup)
    } else {
        (source, [None, None])
    };
    push(&mut slots, cleanup)?;
    reserve(std::mem::size_of::<CacheCleanupFailure>(), io)?;
    Ok(FetchError::Cache {
        path,
        source: std::io::Error::new(
            kind,
            CacheCleanupFailure {
                original,
                cleanup: slots,
            },
        ),
    })
}

pub(super) fn retained_error(
    path: &Path,
    source: std::io::Error,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<FetchError, FetchError> {
    error(
        &source,
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| shell::limit("diagnostic bytes"))?,
        io,
    )?;
    Ok(FetchError::Cache {
        path: path.to_path_buf(),
        source,
    })
}
