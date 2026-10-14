use crate::net::FetchError;
use census_store::{CaptureError, RecoveryArchiveIo, RecoveryFsOperation as Op};
use std::fs::{self, File, Metadata, OpenOptions, ReadDir};
use std::io::{self, Write};
use std::path::Path;

pub(super) fn limit(resource: &'static str) -> FetchError {
    CaptureError::ReadBudgetExceeded { resource }.into()
}

fn admit(path: &Path, operation: Op, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    io.path_bytes(path.as_os_str().len())?;
    io.reserve_owned(
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| limit("diagnostics"))?,
    )?;
    Ok(io.direct_operation(operation)?)
}

fn finish<T>(
    path: &Path,
    result: io::Result<T>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<T, FetchError> {
    match result {
        Ok(value) => {
            io.release_owned(
                path.as_os_str()
                    .len()
                    .checked_add(512)
                    .ok_or_else(|| limit("diagnostics"))?,
            )?;
            Ok(value)
        }
        Err(source) => {
            super::diagnostic::error(
                &source,
                path.as_os_str()
                    .len()
                    .checked_add(512)
                    .ok_or_else(|| limit("diagnostic bytes"))?,
                io,
            )?;
            Err(FetchError::Cache {
                path: path.to_path_buf(),
                source,
            })
        }
    }
}

pub(super) fn invalid(
    path: &Path,
    detail: &'static str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<FetchError, FetchError> {
    io.path_bytes(path.as_os_str().len())?;
    let length = path
        .as_os_str()
        .len()
        .checked_add(detail.len())
        .and_then(|n| n.checked_add(512))
        .ok_or_else(|| limit("diagnostics"))?;
    if length > 8192 {
        return Err(limit("diagnostics"));
    }
    io.diagnostic_bytes(length)?;
    io.reserve_owned(length)?;
    Ok(FetchError::Cache {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, detail),
    })
}

pub(super) fn kind(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Option<Metadata>, FetchError> {
    admit(path, Op::Inspect, io)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => finish(path, Ok(Some(metadata)), io),
        Err(error) if error.kind() == io::ErrorKind::NotFound => finish(path, Ok(None), io),
        Err(error) => finish(path, Err(error), io),
    }
}

pub(super) fn regular(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Option<Metadata>, FetchError> {
    match kind(path, io)? {
        Some(metadata) if metadata.is_file() => Ok(Some(metadata)),
        Some(_) => Err(invalid(path, "capture path is not a regular file", io)?),
        None => Ok(None),
    }
}

pub(super) fn open(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<File, FetchError> {
    admit(path, Op::Open, io)?;
    finish(path, File::open(path), io)
}

pub(super) fn metadata(
    file: &File,
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Metadata, FetchError> {
    admit(path, Op::Inspect, io)?;
    finish(path, file.metadata(), io)
}

pub(super) fn create(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<File, FetchError> {
    admit(path, Op::Create, io)?;
    finish(
        path,
        OpenOptions::new().write(true).create_new(true).open(path),
        io,
    )
}

pub(super) fn write(
    file: &mut File,
    path: &Path,
    bytes: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    admit(path, Op::Create, io)?;
    finish(path, file.write_all(bytes), io)
}

pub(super) fn sync(
    file: &File,
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    admit(path, Op::Sync, io)?;
    finish(path, file.sync_all(), io)
}

pub(super) fn sync_directory(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    let file = open(path, io)?;
    sync(&file, path, io)
}

pub(super) fn create_dir(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    admit(path, Op::Create, io)?;
    finish(path, fs::create_dir(path), io)
}

pub(super) fn read_dir(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<ReadDir, FetchError> {
    io.reserve_owned(
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| limit("allocation"))?,
    )?;
    admit(path, Op::Open, io)?;
    finish(path, fs::read_dir(path), io)
}

pub(super) fn next(
    entries: &mut ReadDir,
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Option<fs::DirEntry>, FetchError> {
    io.directory_next()?;
    admit(path, Op::Inspect, io)?;
    match entries.next() {
        Some(Ok(entry)) => finish(path, Ok(Some(entry)), io),
        Some(Err(error)) => finish(path, Err(error), io),
        None => finish(path, Ok(None), io),
    }
}

pub(super) fn entry_path(
    entry: fs::DirEntry,
    directory: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<std::path::PathBuf, FetchError> {
    io.reserve_owned(4096)?;
    let name = entry.file_name();
    io.path_bytes(name.len())?;
    let path = super::paths::child(directory, &name, io);
    io.release_owned(4096)?;
    path
}

pub(super) fn link(
    source: &Path,
    destination: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    io.path_bytes(source.as_os_str().len())?;
    admit(destination, Op::Link, io)?;
    finish(destination, fs::hard_link(source, destination), io)
}

pub(super) fn rename(
    source: &Path,
    destination: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    io.path_bytes(source.as_os_str().len())?;
    admit(destination, Op::Rename, io)?;
    finish(destination, fs::rename(source, destination), io)
}

pub(super) fn remove_file(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    admit(path, Op::Remove, io)?;
    match fs::remove_file(path) {
        Ok(()) => finish(path, Ok(()), io),
        Err(error) if error.kind() == io::ErrorKind::NotFound => finish(path, Ok(()), io),
        Err(error) => finish(path, Err(error), io),
    }
}

pub(super) fn remove_dir(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    admit(path, Op::Remove, io)?;
    finish(path, fs::remove_dir(path), io)
}

pub(super) fn freeze(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    let file = open(path, io)?;
    let mut permissions = metadata(&file, path, io)?.permissions();
    if !permissions.readonly() {
        permissions.set_readonly(true);
        admit(path, Op::Create, io)?;
        finish(path, file.set_permissions(permissions), io)?;
    }
    sync(&file, path, io)
}

pub(super) fn read_chunk(
    file: &mut File,
    path: &Path,
    bytes: usize,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    use std::io::Read;
    admit(path, Op::Open, io)?;
    let result = file.read_exact(
        io.scratch()
            .get_mut(..bytes)
            .ok_or_else(|| limit("scratch"))?,
    );
    finish(path, result, io)
}

pub(super) fn copy_chunk(
    file: &mut File,
    path: &Path,
    bytes: usize,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    admit(path, Op::Create, io)?;
    let result = file.write_all(io.scratch().get(..bytes).ok_or_else(|| limit("scratch"))?);
    finish(path, result, io)
}

pub(super) fn read_eof(
    file: &mut File,
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<bool, FetchError> {
    use std::io::Read;
    admit(path, Op::Open, io)?;
    let result = match file.read_exact(io.scratch().get_mut(..1).ok_or_else(|| limit("scratch"))?) {
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(true),
        Err(error) => Err(error),
        Ok(()) => Ok(false),
    };
    finish(path, result, io)
}

pub(super) fn release_directory(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    Ok(io.release_owned(
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| limit("allocation"))?,
    )?)
}

pub(super) fn retained_error(
    path: &Path,
    source: std::io::Error,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<FetchError, FetchError> {
    super::diagnostic::error(
        &source,
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| limit("diagnostic bytes"))?,
        io,
    )?;
    Ok(FetchError::Cache {
        path: path.to_path_buf(),
        source,
    })
}
