use super::codec::is_digest;
use super::manifest::FrozenManifest;
use super::objects;
use super::{invalid, ReportError, ReportResult};
use crate::report::io_error;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const POINTER: &str = "root.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Retained {
    pub(super) attempt: String,
    pub(super) manifest: String,
}

impl Retained {
    pub(super) fn validate(&self) -> ReportResult<()> {
        if !is_digest(&self.attempt) || !is_digest(&self.manifest) {
            return Err(invalid(
                "a retained frozen root needs sha256 digests for its attempt and manifest",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenRoot {
    directory: PathBuf,
    manifest: FrozenManifest,
    manifest_digest: String,
    attempt: String,
    bytes: u64,
}

impl FrozenRoot {
    pub(super) fn new(
        directory: PathBuf,
        manifest: FrozenManifest,
        manifest_digest: String,
        attempt: String,
        bytes: u64,
    ) -> Self {
        Self {
            directory,
            manifest,
            manifest_digest,
            attempt,
            bytes,
        }
    }

    pub fn manifest(&self) -> &FrozenManifest {
        &self.manifest
    }

    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    pub fn attempt(&self) -> &str {
        &self.attempt
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

pub(super) fn read_retained(directory: &Path) -> ReportResult<Option<Retained>> {
    let path = directory.join(POINTER);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io_error(&path, source)),
    }
    let retained: Retained = super::io::read_json(&path)?;
    retained.validate()?;
    Ok(Some(retained))
}

pub(super) fn write_retained(directory: &Path, retained: &Retained) -> ReportResult<()> {
    retained.validate()?;
    let path = directory.join(POINTER);
    let encoded = serde_json::to_vec(retained).map_err(|source| ReportError::Decode {
        path: path.clone(),
        line: 0,
        source,
    })?;
    let staging = objects::staging_path(directory, "root")?;
    let published =
        objects::write_staged(&staging, &encoded).and_then(|()| publish(&staging, &path));
    super::io::remove_owned(&staging, published)
}

pub(super) fn remove_retained(directory: &Path) -> ReportResult<()> {
    let path = directory.join(POINTER);
    match std::fs::remove_file(&path) {
        Ok(()) => objects::sync(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(&path, source)),
    }
}

pub(super) fn payload_bytes_of(manifest: &FrozenManifest) -> ReportResult<u64> {
    manifest.streams.iter().try_fold(0_u64, |total, reference| {
        total
            .checked_add(reference.bytes)
            .ok_or_else(|| invalid("the frozen payload byte count overflowed u64"))
    })
}

fn publish(staging: &Path, path: &Path) -> ReportResult<()> {
    std::fs::rename(staging, path).map_err(|source| io_error(path, source))?;
    #[cfg(test)]
    super::freeze::checkpoint("root-renamed")?;
    let directory = path
        .parent()
        .ok_or_else(|| invalid("a retained root path has no directory"))?;
    objects::sync(directory)?;
    #[cfg(test)]
    super::freeze::checkpoint("root-directory-synced")?;
    Ok(())
}
