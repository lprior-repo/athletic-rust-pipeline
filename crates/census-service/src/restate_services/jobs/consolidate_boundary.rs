use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use census_store::Store;

use crate::restate_services::wire::ConsolidatedTable;
use crate::restate_services::JobError;

mod config;
mod evidence;
mod marker;
#[cfg(test)]
mod tests;

const BOUNDARY_ENVIRONMENT: &str = "CENSUS_NATIVE_CONSOLIDATE_BOUNDARY";
const MARKER_ENVIRONMENT: &str = "CENSUS_NATIVE_CONSOLIDATE_MARKER";
const HOLD_ENVIRONMENT: &str = "CENSUS_NATIVE_CONSOLIDATE_HOLD_MILLIS";
const OPERATION_PREFIX: &str = "consolidate:";
const PHASE: &str = "consolidate_durable_before_reply";
const SCHEMA: u8 = 1;

static FIRED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub(crate) enum BoundaryError {
    #[error("native consolidate boundary: invalid arming: {0}")]
    Arming(String),
    #[error("native consolidate boundary: invalid marker path: {0}")]
    Marker(String),
    #[error("native consolidate boundary: invalid hold configuration: {0}")]
    HoldConfig(String),
    #[error("native consolidate boundary: a hold of {0} ms is outside its supported range")]
    Hold(u64),
    #[error(
        "native consolidate boundary: the reached-boundary artifact {} already exists: clear it before arming",
        path.display()
    )]
    Artifact { path: PathBuf },
    #[error("native consolidate boundary: {action} {}: {source}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("native consolidate boundary: marker encoding failed: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("native consolidate boundary: boundary evidence allocation failed")]
    Allocation,
    #[error("native consolidate boundary: a snapshot read exceeds its digest buffer")]
    Digest,
    #[error("native consolidate boundary: the marker path has no parent directory")]
    Parent,
    #[error(
        "native consolidate boundary: {primary}; the pending marker cleanup also failed: {cleanup}"
    )]
    Cleanup {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
    #[error(
        "native consolidate boundary: operation {operation} reached its boundary but no fault was delivered within {millis} ms"
    )]
    HoldExpired { operation: String, millis: u64 },
}

impl From<BoundaryError> for JobError {
    fn from(error: BoundaryError) -> Self {
        Self::Terminal {
            message: error.to_string(),
        }
    }
}

pub(crate) fn operation(key: &str) -> String {
    format!("{OPERATION_PREFIX}{key}")
}

#[tracing::instrument(skip_all)]
pub(crate) fn reached(
    store: &Store,
    operation: &str,
    tables: &[ConsolidatedTable],
) -> Result<(), BoundaryError> {
    let Some(armed) = config::armed()? else {
        return Ok(());
    };
    if armed != operation {
        return Ok(());
    }
    let path = config::marker_path()?;
    let hold = config::hold_millis()?;
    if FIRED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    if let Err(error) = publish_reached(store, operation, tables, &path) {
        FIRED.store(false, Ordering::SeqCst);
        return Err(error);
    }
    tracing::info!(
        phase = PHASE,
        operation,
        marker = %path.display(),
        hold_millis = hold,
        "native consolidate boundary reached before acknowledgement"
    );
    config::deliver(operation, hold)
}

fn publish_reached(
    store: &Store,
    operation: &str,
    tables: &[ConsolidatedTable],
    path: &Path,
) -> Result<(), BoundaryError> {
    let reached = marker::Marker::new(operation, evidence::gather(store, tables)?);
    marker::publish(path, &reached)
}

pub(super) fn io(action: &'static str, path: &Path, source: std::io::Error) -> BoundaryError {
    BoundaryError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}
