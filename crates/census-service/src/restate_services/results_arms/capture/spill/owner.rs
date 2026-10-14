use std::fs::{File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use census_store::{StoreError, StoreResult};

use crate::restate_services::results_arms::budget::{self, SpillAdmission};

mod error;

pub(super) use error::{io_error, refused};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}

struct OwnedName {
    path: PathBuf,
    identity: Identity,
    bytes: Option<u64>,
}

enum Ownership {
    Named,
    Unlinked,
}

pub(super) struct Directory {
    path: PathBuf,
    identity: Identity,
    names: Vec<OwnedName>,
    admission: Arc<SpillAdmission>,
    ownership: Ownership,
}

impl Directory {
    pub(super) fn create(root: &Path, admission: Arc<SpillAdmission>) -> StoreResult<Self> {
        supported()?;
        if root.as_os_str().len() > 4096 {
            return Err(refused("results spill root exceeds path budget"));
        }
        let path = root.join(".results-source-spill");
        let mut names = Vec::new();
        names
            .try_reserve_exact(budget::SPILL_FILES)
            .map_err(|_| budget::allocation())?;
        if let Err(source) = create_private(&path) {
            admission.poison()?;
            return Err(StoreError::Refused {
                detail: format!(
                    "results spill namespace requires verified cleanup at {}: {source}",
                    path.display()
                ),
            });
        }
        Self::finish_created(path, names, admission)
    }

    fn finish_created(
        path: PathBuf,
        names: Vec<OwnedName>,
        admission: Arc<SpillAdmission>,
    ) -> StoreResult<Self> {
        let ownership = std::fs::symlink_metadata(&path)
            .map_err(|source| io_error(&path, source))
            .and_then(|metadata| directory_identity(&metadata));
        let identity = match ownership {
            Ok(identity) => identity,
            Err(primary) => {
                let poison = admission.poison();
                return Err(StoreError::Refused { detail: format!("results spill namespace ownership unresolved: {primary}; disk poisoning: {poison:?}") });
            }
        };
        Ok(Self {
            path,
            identity,
            names,
            admission,
            ownership: Ownership::Named,
        })
    }

    pub(super) fn create_file(&mut self, name: &str) -> StoreResult<(PathBuf, File)> {
        if self.names.len() >= budget::SPILL_FILES {
            return Err(refused("results spill file budget exhausted"));
        }
        let path = self.path.join(name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| io_error(&path, source))?;
        let identity = identity(&file.metadata().map_err(|source| io_error(&path, source))?)?;
        self.names.push(OwnedName {
            path: path.clone(),
            identity,
            bytes: None,
        });
        Ok((path, file))
    }

    pub(super) fn seal_name(&mut self, path: &Path, file: &File, bytes: u64) -> StoreResult<()> {
        let held = file.metadata().map_err(|source| io_error(path, source))?;
        let owned = self
            .names
            .iter_mut()
            .find(|owned| owned.path == path)
            .ok_or_else(|| refused("results spill sealed name has no ownership"))?;
        if identity(&held)? != owned.identity || held.len() != bytes {
            return Err(refused(
                "results spill sealed handle differs from owned name",
            ));
        }
        owned.bytes = Some(bytes);
        verify_owned(owned)
    }

    pub(super) fn abort(mut self, primary: StoreError) -> StoreError {
        match self.cleanup() {
            Ok(()) => {
                self.ownership = Ownership::Unlinked;
                primary
            }
            Err(cleanup) => {
                let poison = self.admission.poison();
                StoreError::Refused { detail: format!("results spill operation failed: {primary}; cleanup failed: {cleanup}; disk poisoning: {poison:?}") }
            }
        }
    }

    pub(super) fn unlink(mut self) -> StoreResult<()> {
        match self.cleanup() {
            Ok(()) => {
                self.ownership = Ownership::Unlinked;
                Ok(())
            }
            Err(primary) => match self.admission.poison() {
                Ok(()) => Err(primary),
                Err(poison) => Err(StoreError::Refused {
                    detail: format!(
                        "results spill unlink failed: {primary}; poisoning failed: {poison}"
                    ),
                }),
            },
        }
    }

    fn cleanup(&self) -> StoreResult<()> {
        self.verify_directory()?;
        self.verify_names()?;
        self.names.iter().try_for_each(remove_owned)?;
        std::fs::remove_dir(&self.path).map_err(|source| io_error(&self.path, source))
    }

    fn verify_directory(&self) -> StoreResult<()> {
        let metadata =
            std::fs::symlink_metadata(&self.path).map_err(|source| io_error(&self.path, source))?;
        if directory_identity(&metadata)? != self.identity {
            return Err(refused("results spill namespace changed identity"));
        }
        Ok(())
    }

    fn verify_names(&self) -> StoreResult<()> {
        let entries =
            std::fs::read_dir(&self.path).map_err(|source| io_error(&self.path, source))?;
        entries
            .take(budget::SPILL_FILES.saturating_add(1))
            .enumerate()
            .try_for_each(|(index, entry)| {
                if index >= budget::SPILL_FILES {
                    return Err(refused("results spill namespace has unexpected entries"));
                }
                let entry = entry.map_err(|source| io_error(&self.path, source))?;
                self.names
                    .iter()
                    .find(|owned| owned.path == entry.path())
                    .ok_or_else(|| refused("results spill namespace contains a foreign name"))
                    .and_then(verify_owned)
            })
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        if matches!(self.ownership, Ownership::Unlinked) {
            return;
        }
        if let Err(cleanup) = self.cleanup() {
            if let Err(poison) = self.admission.poison() {
                tracing::error!(%cleanup, %poison, "results spill cleanup and poisoning failed");
            } else {
                tracing::error!(%cleanup, "results spill cleanup failed; disk admission remains poisoned");
            }
        }
    }
}

pub(super) fn reopen(path: &Path, expected: Identity, bytes: u64) -> StoreResult<File> {
    let named = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if identity(&named)? != expected || named.len() != bytes {
        return Err(refused(
            "results spill name changed before read-only reopen",
        ));
    }
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    let held = file.metadata().map_err(|source| io_error(path, source))?;
    if identity(&held)? != expected || held.len() != bytes {
        return Err(refused("results spill read-only handle changed ownership"));
    }
    Ok(file)
}

fn verify_owned(owned: &OwnedName) -> StoreResult<()> {
    let metadata =
        std::fs::symlink_metadata(&owned.path).map_err(|source| io_error(&owned.path, source))?;
    if identity(&metadata)? != owned.identity
        || owned.bytes.is_some_and(|bytes| bytes != metadata.len())
    {
        return Err(refused("results spill owned name changed identity"));
    }
    Ok(())
}

fn remove_owned(owned: &OwnedName) -> StoreResult<()> {
    match std::fs::symlink_metadata(&owned.path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(&owned.path, source)),
        Ok(_) => {
            verify_owned(owned)?;
            std::fs::remove_file(&owned.path).map_err(|source| io_error(&owned.path, source))
        }
    }
}

fn directory_identity(metadata: &Metadata) -> StoreResult<Identity> {
    if !metadata.file_type().is_dir() {
        return Err(refused("results spill namespace is not an owned directory"));
    }
    platform_identity(metadata)
}

pub(super) fn identity(metadata: &Metadata) -> StoreResult<Identity> {
    if !metadata.file_type().is_file() {
        return Err(refused("results spill backing is not a regular file"));
    }
    platform_identity(metadata)
}

#[cfg(unix)]
fn platform_identity(metadata: &Metadata) -> StoreResult<Identity> {
    use std::os::unix::fs::MetadataExt;
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn platform_identity(_metadata: &Metadata) -> StoreResult<Identity> {
    Err(refused(
        "results spill requires supported Unix file ownership",
    ))
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private(_path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "results spill requires Unix",
    ))
}

#[cfg(unix)]
fn supported() -> StoreResult<()> {
    Ok(())
}

#[cfg(not(unix))]
fn supported() -> StoreResult<()> {
    Err(refused(
        "results spill requires supported Unix anonymous-file semantics",
    ))
}

#[cfg(all(test, unix))]
#[path = "owner_tests.rs"]
mod tests;
