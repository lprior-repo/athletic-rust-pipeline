use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::{StoreError, StoreResult};

#[derive(Clone, Copy, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub(crate) enum Position {
    SourceBatchStagedBeforeCommit { ordinal: u64 },
    SourceChunkCommittedBeforeNext { ordinal: u64 },
    DerivedBatchStagedBeforePublish { generation: u64 },
}

impl Position {
    fn phase(self) -> &'static str {
        match self {
            Self::SourceBatchStagedBeforeCommit { .. } => "source_batch_staged_before_commit",
            Self::SourceChunkCommittedBeforeNext { .. } => "source_chunk_committed_before_next",
            Self::DerivedBatchStagedBeforePublish { .. } => "derived_batch_staged_before_publish",
        }
    }
}

#[derive(Serialize)]
struct Marker<'a> {
    schema: u8,
    pid: u32,
    operation: &'a str,
    input_digest: &'a str,
    #[serde(flatten)]
    position: Position,
}

pub(crate) fn pause(position: Position, operation: &str, input_digest: &str) -> StoreResult<()> {
    let Some(path) = marker_path(position)? else {
        return Ok(());
    };
    let marker = Marker {
        schema: 1,
        pid: std::process::id(),
        operation,
        input_digest,
        position,
    };
    publish(&path, &marker)?;
    for _ in 0..600 {
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(StoreError::Refused {
        detail: format!(
            "native worker boundary {} reached but fault was not delivered within 60 seconds",
            position.phase()
        ),
    })
}

fn marker_path(position: Position) -> StoreResult<Option<PathBuf>> {
    let selected = match std::env::var("CENSUS_NATIVE_WORKER_BOUNDARY") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => {
            return Err(StoreError::Refused {
                detail: format!("invalid native boundary configuration: {error}"),
            })
        }
    };
    if !matches!(
        selected.as_str(),
        "source_batch_staged_before_commit"
            | "source_chunk_committed_before_next"
            | "derived_batch_staged_before_publish"
    ) {
        return Err(StoreError::Refused {
            detail: format!("unknown native worker boundary {selected}"),
        });
    }
    if selected != position.phase() {
        return Ok(None);
    }
    let path = std::env::var_os("CENSUS_NATIVE_WORKER_MARKER")
        .map(PathBuf::from)
        .ok_or_else(|| StoreError::Refused {
            detail: "native worker boundary requires an exclusive marker path".into(),
        })?;
    if !path.is_absolute() {
        return Err(StoreError::Refused {
            detail: "native worker marker path must be absolute".into(),
        });
    }
    Ok(Some(path))
}

fn publish(path: &Path, marker: &Marker<'_>) -> StoreResult<()> {
    let pending = path.with_extension("pending");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|source| io_error(&pending, source))?;
    let publication = publish_pending(path, &pending, &mut file, marker);
    let cleanup = std::fs::remove_file(&pending).map_err(|source| io_error(&pending, source));
    match (publication, cleanup) {
        (Ok(()), Ok(())) => sync_parent(path),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(StoreError::Refused {
            detail: format!(
                "native marker publication failed: {error}; pending cleanup also failed: {cleanup}"
            ),
        }),
    }
}

fn publish_pending(
    path: &Path,
    pending: &Path,
    file: &mut std::fs::File,
    marker: &Marker<'_>,
) -> StoreResult<()> {
    serde_json::to_writer(&mut *file, marker).map_err(|source| StoreError::Json {
        detail: "serializing reached native worker boundary".into(),
        source,
    })?;
    file.sync_all()
        .map_err(|source| io_error(pending, source))?;
    std::fs::hard_link(pending, path).map_err(|source| io_error(path, source))?;
    sync_parent(path)
}

fn sync_parent(path: &Path) -> StoreResult<()> {
    let parent = path.parent().ok_or_else(|| StoreError::Refused {
        detail: "native marker has no parent directory".into(),
    })?;
    crate::fs::fsync_dir(parent)
}

fn io_error(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
