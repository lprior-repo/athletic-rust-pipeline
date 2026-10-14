use super::shell;
use crate::net::FetchError;
use census_domain::model::PreparedCaptureInputV1;
use census_store::RecoveryArchiveIo;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub(super) struct ArchivePaths {
    pub(super) archive: PathBuf,
    pub(super) body: PathBuf,
    pub(super) metadata: PathBuf,
    pub(super) reservations: PathBuf,
}

impl ArchivePaths {
    pub(super) fn new(
        root: &Path,
        prepared: &PreparedCaptureInputV1,
        io: &mut RecoveryArchiveIo<'_>,
    ) -> Result<Self, FetchError> {
        let archive = child(root, "archive", io)?;
        let bodies = child(&archive, "bodies", io)?;
        let captures = child(&archive, "captures", io)?;
        let content = child(&captures, prepared.content_digest().as_str(), io)?;
        let body = suffixed(&bodies, prepared.content_digest().as_str(), ".body", io)?;
        let metadata = suffixed(
            &content,
            prepared.metadata_digest().as_str(),
            ".meta.json",
            io,
        )?;
        let reservations = child(&archive, "reservations", io)?;
        io.release_owned(bodies.as_os_str().len())?;
        io.release_owned(captures.as_os_str().len())?;
        io.release_owned(content.as_os_str().len())?;
        Ok(Self {
            archive,
            body,
            metadata,
            reservations,
        })
    }
}

pub(super) fn child(
    parent: &Path,
    name: impl AsRef<OsStr>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PathBuf, FetchError> {
    let name = name.as_ref();
    let length = parent
        .as_os_str()
        .len()
        .checked_add(name.len())
        .and_then(|length| length.checked_add(1))
        .ok_or_else(|| shell::limit("archive path bytes"))?;
    io.path_bytes(length)?;
    io.reserve_owned(length)?;
    let mut value = OsString::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| shell::limit("allocation"))?;
    value.push(parent.as_os_str());
    value.push(OsStr::from_bytes(b"/"));
    value.push(name);
    Ok(PathBuf::from(value))
}

pub(super) fn suffixed(
    parent: &Path,
    name: &str,
    suffix: &str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PathBuf, FetchError> {
    let length = name
        .len()
        .checked_add(suffix.len())
        .ok_or_else(|| shell::limit("archive path bytes"))?;
    io.path_bytes(length)?;
    io.reserve_owned(length)?;
    let mut value = OsString::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| shell::limit("allocation"))?;
    value.push(name);
    value.push(suffix);
    let result = child(parent, &value, io);
    io.release_owned(length)?;
    result
}

pub(super) fn safe_directory(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    match shell::kind(path, io)? {
        Some(metadata) if metadata.is_dir() => Ok(()),
        Some(_) => Err(shell::invalid(
            path,
            "archive parent is not a regular directory",
            io,
        )?),
        None => {
            shell::create_dir(path, io)?;
            shell::sync_directory(path, io)?;
            let parent = path
                .parent()
                .ok_or_else(|| shell::limit("archive ancestry"))?;
            shell::sync_directory(parent, io)
        }
    }
}

pub(super) fn initialize(
    root: &Path,
    paths: &ArchivePaths,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    safe_directory(&paths.archive, io)?;
    let bodies = paths
        .body
        .parent()
        .ok_or_else(|| shell::limit("archive ancestry"))?;
    safe_directory(bodies, io)?;
    let content = paths
        .metadata
        .parent()
        .ok_or_else(|| shell::limit("archive ancestry"))?;
    let captures = content
        .parent()
        .ok_or_else(|| shell::limit("archive ancestry"))?;
    safe_directory(captures, io)?;
    safe_directory(content, io)?;
    safe_directory(&paths.reservations, io)?;
    match shell::kind(root, io)? {
        Some(metadata) if metadata.is_dir() => Ok(()),
        _ => Err(shell::invalid(root, "archive root changed", io)?),
    }
}
