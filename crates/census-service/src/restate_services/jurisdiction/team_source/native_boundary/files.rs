use std::fs::{self, File, Metadata, OpenOptions};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use super::error::{artifact, io, BoundaryError};

mod validation;
use validation::{open_flags, same_file, unchanged, validate_path};

pub(super) const MAX_BYTES: usize = 4096;
pub(super) const MAX_MARKER_BYTES: usize = 32 * 1024 * 1024;
const LINUX_DIRECTORY: i32 = 0o200000;

pub(super) struct Directory {
    handle: File,
    path: PathBuf,
    uid: u32,
}

impl Directory {
    pub(super) fn open(config: &Path) -> Result<Self, BoundaryError> {
        validate_path(config)?;
        let parent = config
            .parent()
            .ok_or(BoundaryError::Path("missing parent"))?;
        let canonical =
            fs::canonicalize(parent).map_err(|source| io("resolving directory", parent, source))?;
        if canonical != parent {
            return Err(artifact(parent, "symlink or noncanonical directory"));
        }
        let uid = fs::metadata("/proc/self")
            .map_err(|source| io("reading effective owner", Path::new("/proc/self"), source))?
            .uid();
        let before = fs::symlink_metadata(parent)
            .map_err(|source| io("inspecting directory", parent, source))?;
        if !before.file_type().is_dir() || before.uid() != uid || before.mode() & 0o777 != 0o700 {
            return Err(artifact(
                parent,
                "directory must be nonsymlink, process-owned and mode 0700",
            ));
        }
        let handle = OpenOptions::new()
            .read(true)
            .custom_flags(open_flags()? | LINUX_DIRECTORY)
            .open(parent)
            .map_err(|source| io("opening directory", parent, source))?;
        let after = handle
            .metadata()
            .map_err(|source| io("checking directory handle", parent, source))?;
        if !unchanged(&before, &after) {
            return Err(artifact(parent, "directory changed during open"));
        }
        Ok(Self {
            handle,
            path: parent.to_path_buf(),
            uid,
        })
    }

    pub(super) fn child(&self, name: &std::ffi::OsStr) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.handle.as_raw_fd())).join(name)
    }

    pub(super) fn inspect(&self, path: &Path) -> Result<Option<Metadata>, BoundaryError> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                self.validate_file(path, &metadata)?;
                Ok(Some(metadata))
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io("inspecting file", path, source)),
        }
    }

    pub(super) fn read<T: DeserializeOwned>(&self, path: &Path) -> Result<T, BoundaryError> {
        self.read_bounded(path, MAX_BYTES)
    }

    pub(super) fn read_marker<T: DeserializeOwned>(&self, path: &Path) -> Result<T, BoundaryError> {
        self.read_bounded(path, MAX_MARKER_BYTES)
    }

    fn read_bounded<T: DeserializeOwned>(
        &self,
        path: &Path,
        max: usize,
    ) -> Result<T, BoundaryError> {
        let before = self
            .inspect(path)?
            .ok_or_else(|| artifact(path, "file is absent"))?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(open_flags()?)
            .open(path)
            .map_err(|source| io("opening file", path, source))?;
        let opened = file
            .metadata()
            .map_err(|source| io("checking opened file", path, source))?;
        self.validate_file(path, &opened)?;
        if !unchanged(&before, &opened) {
            return Err(artifact(path, "file changed during open"));
        }
        let length = usize::try_from(opened.len())
            .map_err(|_| artifact(path, "file length exceeds address space"))?;
        if length > max {
            return Err(artifact(path, "file exceeds native artifact bound"));
        }
        let capacity = length
            .checked_add(1)
            .ok_or_else(|| artifact(path, "file read limit overflows"))?;
        let limit =
            u64::try_from(capacity).map_err(|_| artifact(path, "file read limit exceeds u64"))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(capacity)?;
        (&file)
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|source| io("reading bounded file", path, source))?;
        if bytes.len() != length {
            return Err(artifact(path, "file length changed during read"));
        }
        let after = file
            .metadata()
            .map_err(|source| io("checking read file", path, source))?;
        let current = self
            .inspect(path)?
            .ok_or_else(|| artifact(path, "file disappeared during read"))?;
        if !unchanged(&opened, &after) || !unchanged(&after, &current) {
            return Err(artifact(path, "file changed during read"));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub(super) fn create_pending(&self, path: &Path) -> Result<File, BoundaryError> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(open_flags()?)
            .open(path)
            .map_err(|source| io("creating owned pending file", path, source))?;
        Ok(file)
    }

    pub(super) fn verify_owned(&self, path: &Path, file: &File) -> Result<(), BoundaryError> {
        let owned = file
            .metadata()
            .map_err(|source| io("checking owned file", path, source))?;
        let current = self
            .inspect(path)?
            .ok_or_else(|| artifact(path, "owned file disappeared"))?;
        if !same_file(&owned, &current) {
            return Err(artifact(path, "file no longer belongs to this worker"));
        }
        Ok(())
    }

    pub(super) fn remove_owned(&self, path: &Path, file: &File) -> Result<(), BoundaryError> {
        self.verify_owned(path, file)?;
        fs::remove_file(path).map_err(|source| io("removing owned pending file", path, source))
    }

    pub(super) fn sync_file(&self, path: &Path) -> Result<(), BoundaryError> {
        let before = self
            .inspect(path)?
            .ok_or_else(|| artifact(path, "marker disappeared"))?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(open_flags()?)
            .open(path)
            .map_err(|source| io("opening retained marker", path, source))?;
        self.verify_owned(path, &file)?;
        let after = file
            .metadata()
            .map_err(|source| io("checking retained marker", path, source))?;
        if !unchanged(&before, &after) {
            return Err(artifact(path, "retained marker changed during open"));
        }
        file.sync_all()
            .map_err(|source| io("syncing retained marker", path, source))
    }

    pub(super) fn sync(&self) -> Result<(), BoundaryError> {
        let metadata = self
            .handle
            .metadata()
            .map_err(|source| io("checking private directory", &self.path, source))?;
        if metadata.uid() != self.uid || metadata.mode() & 0o777 != 0o700 {
            return Err(artifact(
                &self.path,
                "private directory ownership or mode changed",
            ));
        }
        self.handle
            .sync_all()
            .map_err(|source| io("syncing private directory", &self.path, source))
    }

    fn validate_file(&self, path: &Path, metadata: &Metadata) -> Result<(), BoundaryError> {
        if !metadata.file_type().is_file()
            || metadata.uid() != self.uid
            || metadata.mode() & 0o022 != 0
        {
            return Err(artifact(
                path,
                "file must be regular, nonsymlink, process-owned and not group/world writable",
            ));
        }
        if usize::try_from(metadata.len()).map_err(|_| {
            artifact(
                path,
                "native marker length exceeds the supported address space",
            )
        })? > MAX_MARKER_BYTES
        {
            return Err(artifact(
                path,
                "file exceeds bounded native marker byte budget",
            ));
        }
        Ok(())
    }
}
