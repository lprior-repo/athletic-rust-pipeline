use super::{CaptureTransportError, FetchError};
use std::fmt::{self, Display};
use std::path::PathBuf;

#[derive(Debug)]
pub enum RecoveryFailurePrimary {
    Store(Box<census_store::StoreError>),
    Capture(CaptureTransportError),
    Encode {
        target: String,
        source: serde_json::Error,
    },
}

#[derive(Debug)]
pub enum RecoveryCleanupLeaf {
    Store(Box<census_store::StoreError>),
    Cache {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct RecoveryCleanupFailure {
    pub(in crate::net) primary: RecoveryFailurePrimary,
    pub(in crate::net) cleanup: [Option<RecoveryCleanupLeaf>; 2],
}

impl RecoveryCleanupFailure {
    pub fn primary(&self) -> &RecoveryFailurePrimary {
        &self.primary
    }
    pub fn cleanup(&self) -> &[Option<RecoveryCleanupLeaf>; 2] {
        &self.cleanup
    }
    pub(in crate::net) fn from_parts(
        primary: RecoveryFailurePrimary,
        cleanup: [Option<RecoveryCleanupLeaf>; 2],
    ) -> Self {
        Self { primary, cleanup }
    }
}

impl Display for RecoveryFailurePrimary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => Display::fmt(error, formatter),
            Self::Capture(error) => Display::fmt(error, formatter),
            Self::Encode { target, source } => {
                write!(formatter, "json encode failed for {target}: {source}")
            }
        }
    }
}

impl std::error::Error for RecoveryFailurePrimary {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(error) => Some(error.as_ref()),
            Self::Capture(error) => Some(error),
            Self::Encode { source, .. } => Some(source),
        }
    }
}

impl Display for RecoveryCleanupLeaf {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => Display::fmt(error, formatter),
            Self::Cache { path, source } => {
                write!(formatter, "cache i/o for {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for RecoveryCleanupLeaf {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(error) => Some(error.as_ref()),
            Self::Cache { source, .. } => Some(source),
        }
    }
}

impl Display for RecoveryCleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "original recovery error: {}", self.primary)?;
        for error in self.cleanup.iter().flatten() {
            write!(formatter, "; cleanup error: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RecoveryCleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.primary)
    }
}

impl FetchError {
    pub fn recovery_primary(&self) -> Option<&RecoveryFailurePrimary> {
        match self {
            Self::RecoveryCleanup(error) => Some(error.primary()),
            _ => None,
        }
    }
}
