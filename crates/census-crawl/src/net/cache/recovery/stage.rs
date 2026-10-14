use super::{paths, shell};
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use std::path::{Path, PathBuf};

pub(super) fn create(root: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<PathBuf, FetchError> {
    directory(root, ".capture-stage", io)
}

pub(super) fn directory(
    root: &Path,
    prefix: &str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PathBuf, FetchError> {
    use std::fmt::Write;
    let diagnostics = root
        .as_os_str()
        .len()
        .checked_add(512)
        .ok_or_else(|| shell::limit("diagnostics"))?;
    io.path_bytes(root.as_os_str().len())?;
    io.reserve_owned(diagnostics)?;
    let (sequence, time) = match super::super::archive_io::stage_identity(root) {
        Ok(identity) => identity,
        Err(error) => {
            super::diagnostic::error(&error, 512, io)?;
            return Err(error);
        }
    };
    io.release_owned(diagnostics)?;
    io.reserve_owned(128)?;
    let mut name = String::new();
    name.try_reserve_exact(128)
        .map_err(|_| shell::limit("allocation"))?;
    write!(
        &mut name,
        "{prefix}-{}-{time}-{sequence}",
        std::process::id()
    )
    .map_err(|_| shell::limit("stage encoding"))?;
    let stage = paths::child(root, &name, io)?;
    io.release_owned(128)?;
    shell::create_dir(&stage, io)?;
    shell::sync_directory(&stage, io)?;
    shell::sync_directory(root, io)?;
    Ok(stage)
}

pub(super) fn write_leaf(
    stage: &Path,
    name: &str,
    bytes: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PathBuf, FetchError> {
    let path = paths::child(stage, name, io)?;
    let mut file = shell::create(&path, io)?;
    shell::write(&mut file, &path, bytes, io)?;
    shell::sync(&file, &path, io)?;
    shell::freeze(&path, io)?;
    Ok(path)
}

pub(super) fn publish(
    staged: &Path,
    destination: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    shell::link(staged, destination, io)?;
    shell::sync_directory(
        destination
            .parent()
            .ok_or_else(|| shell::limit("archive ancestry"))?,
        io,
    )
}

pub(super) fn cleanup(
    stage: &Path,
    names: &[&str],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    for name in names {
        let path = paths::child(stage, name, io)?;
        shell::remove_file(&path, io)?;
        io.release_owned(path.as_os_str().len())?;
    }
    shell::remove_dir(stage, io)?;
    shell::sync_directory(
        stage
            .parent()
            .ok_or_else(|| shell::limit("stage ancestry"))?,
        io,
    )
}

pub(super) fn fail(
    stage: &Path,
    error: FetchError,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let cleanup = shell::sync_directory(stage, io);
    super::diagnostic::with_cleanup(stage, Err(error), cleanup, io)
}
