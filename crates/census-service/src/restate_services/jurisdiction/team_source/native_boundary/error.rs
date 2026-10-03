use std::path::PathBuf;

use crate::restate_services::JobError;

#[derive(Debug, thiserror::Error)]
pub(super) enum BoundaryError {
    #[error("native source boundary: unsupported schema {0}")]
    Schema(u8),
    #[error("native source boundary: attempt {0} is outside 1..=3")]
    Attempt(u8),
    #[error("native source boundary: timeout {0} is outside 1..=60 seconds")]
    TimeoutBounds(u64),
    #[error("native source boundary: operation must be exact, nonempty, bounded and without wildcards or control characters")]
    Operation,
    #[error("native source boundary: invalid configuration path: {0}")]
    Path(&'static str),
    #[error("native source boundary: unsafe artifact {}: {reason}", path.display())]
    Artifact { path: PathBuf, reason: &'static str },
    #[error("native source boundary: {action} {}: {source}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("native source boundary: invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("native source boundary: bounded input allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("native source boundary: journaled identity is empty or oversized")]
    IdentityBounds,
    #[error("native source boundary: existing marker contradicts the reserved journaled identity")]
    IdentityMismatch,
    #[error("native source boundary: {primary}; owned pending cleanup also failed: {cleanup}")]
    Cleanup {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
    #[error("native source boundary: failed injection for {operation} attempt {attempt}: hold expired after {seconds} seconds without reset")]
    HoldExpired {
        operation: String,
        attempt: u8,
        seconds: u64,
    },
}

impl From<BoundaryError> for JobError {
    fn from(error: BoundaryError) -> Self {
        Self::Terminal {
            message: error.to_string(),
        }
    }
}

pub(super) fn io(
    action: &'static str,
    path: &std::path::Path,
    source: std::io::Error,
) -> BoundaryError {
    BoundaryError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

pub(super) fn artifact(path: &std::path::Path, reason: &'static str) -> BoundaryError {
    BoundaryError::Artifact {
        path: path.to_path_buf(),
        reason,
    }
}
