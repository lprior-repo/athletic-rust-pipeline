use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::evidence::TableEvidence;
use super::{io, BoundaryError, PHASE, SCHEMA};

#[derive(Serialize)]
pub(super) struct Marker {
    schema: u8,
    phase: &'static str,
    pid: u32,
    operation: String,
    tables: Vec<TableEvidence>,
}

impl Marker {
    pub(super) fn new(operation: &str, tables: Vec<TableEvidence>) -> Self {
        Self {
            schema: SCHEMA,
            phase: PHASE,
            pid: std::process::id(),
            operation: operation.to_string(),
            tables,
        }
    }
}

pub(super) fn publish(path: &Path, marker: &Marker) -> Result<(), BoundaryError> {
    let pending = pending_path(path);
    let bytes = serde_json::to_vec(marker)?;
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
    {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(BoundaryError::Artifact { path: pending });
        }
        Err(source) => {
            return Err(io("creating the reached-boundary marker", &pending, source));
        }
    };
    let publication = write_and_link(&mut file, &pending, path, &bytes);
    let cleanup = remove_pending(&pending);
    match (publication, cleanup) {
        (Ok(()), Ok(())) => sync_parent(path),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(BoundaryError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}

fn write_and_link(
    file: &mut std::fs::File,
    pending: &Path,
    target: &Path,
    bytes: &[u8],
) -> Result<(), BoundaryError> {
    file.write_all(bytes)
        .map_err(|source| io("writing the reached-boundary marker", pending, source))?;
    file.sync_all()
        .map_err(|source| io("syncing the reached-boundary marker", pending, source))?;
    match std::fs::hard_link(pending, target) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(BoundaryError::Artifact {
                path: target.to_path_buf(),
            })
        }
        Err(source) => Err(io("publishing the reached-boundary marker", target, source)),
    }
}

fn remove_pending(pending: &Path) -> Result<(), BoundaryError> {
    match std::fs::remove_file(pending) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io(
            "removing the pending reached-boundary marker",
            pending,
            source,
        )),
    }
}

fn sync_parent(path: &Path) -> Result<(), BoundaryError> {
    let parent = path.parent().ok_or(BoundaryError::Parent)?;
    std::fs::File::open(parent)
        .map_err(|source| io("opening the marker directory", parent, source))?
        .sync_all()
        .map_err(|source| io("syncing the marker directory", parent, source))
}

fn pending_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".pending");
    PathBuf::from(name)
}
