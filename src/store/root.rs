use super::{error::Result, StoreError};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::{fs, io::ErrorKind, path::Path};

pub fn prepare_root(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_root(metadata),
        Err(error) if error.kind() == ErrorKind::NotFound => create_root(path),
        Err(_) => Err(StoreError::DatabaseIo),
    }
}

fn create_root(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => fs::symlink_metadata(path)
            .map_err(|_| StoreError::DatabaseIo)
            .and_then(validate_root),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => fs::symlink_metadata(path)
            .map_err(|_| StoreError::DatabaseIo)
            .and_then(validate_root),
        Err(_) => Err(StoreError::DatabaseIo),
    }
}

#[cfg(unix)]
fn validate_root(metadata: fs::Metadata) -> Result<()> {
    (metadata.file_type().is_dir() && metadata.mode() & 0o077 == 0)
        .then_some(())
        .ok_or(StoreError::InsecureRoot)
}

#[cfg(not(unix))]
fn validate_root(metadata: fs::Metadata) -> Result<()> {
    metadata
        .file_type()
        .is_dir()
        .then_some(())
        .ok_or(StoreError::InsecureRoot)
}
