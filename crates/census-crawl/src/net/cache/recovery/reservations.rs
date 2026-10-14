use super::{paths, shell, stream};
use crate::net::FetchError;
use census_store::{RecoveryArchiveIo, RecoveryFsOperation as Op, RecoveryPayloadRole as Role};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn path(
    directory: &Path,
    body: &str,
    metadata: &str,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PathBuf, FetchError> {
    if !super::super::is_valid_hex64(body) || !super::super::is_valid_hex64(metadata) {
        return Err(shell::invalid(
            directory,
            "reservation key is not a pair of SHA256 digests",
            io,
        )?);
    }
    io.reserve_owned(129)?;
    let mut name = String::new();
    name.try_reserve_exact(129)
        .map_err(|_| shell::limit("allocation"))?;
    name.push_str(body);
    name.push('-');
    name.push_str(metadata);
    let result = paths::child(directory, &name, io);
    io.release_owned(129)?;
    result
}

pub(super) fn read(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(u64, u64), FetchError> {
    validate_name(path, io)?;
    let metadata = match shell::regular(path, io)? {
        Some(metadata) => metadata,
        None => return Err(shell::invalid(path, "reservation record is missing", io)?),
    };
    if metadata.len() > 41 {
        return Err(shell::invalid(
            path,
            "reservation record exceeds its limit",
            io,
        )?);
    }
    let size = usize::try_from(metadata.len()).map_err(|_| shell::limit("reservation payload"))?;
    let mut file = shell::open(path, io)?;
    let opened = shell::metadata(&file, path, io)?;
    if !opened.is_file() || opened.len() != metadata.len() {
        return Err(shell::invalid(path, "reservation changed", io)?);
    }
    io.payload(Role::Reservation, metadata.len())?;
    shell::read_chunk(&mut file, path, size, io)?;
    let parsed = match std::str::from_utf8(
        io.scratch()
            .get(..size)
            .ok_or_else(|| shell::limit("scratch"))?,
    ) {
        Ok(text) => super::super::archive::reservations::parse_record(text),
        Err(_) => Err("reservation record is not UTF8"),
    };
    let pair = match parsed {
        Ok(pair) => pair,
        Err(detail) => return Err(shell::invalid(path, detail, io)?),
    };
    stream::eof(&mut file, path, Role::Reservation, io)?;
    Ok(pair)
}

fn validate_name(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err(shell::invalid(path, "reservation key is not UTF8", io)?);
    };
    let Some((body, metadata)) = name.split_once('-') else {
        return Err(shell::invalid(
            path,
            "reservation key requires two SHA256 digests",
            io,
        )?);
    };
    if !super::super::is_valid_hex64(body) || !super::super::is_valid_hex64(metadata) {
        return Err(shell::invalid(
            path,
            "reservation key is not a pair of SHA256 digests",
            io,
        )?);
    }
    Ok(())
}

pub(super) fn scan(
    directory: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(u64, Vec<PathBuf>), FetchError> {
    let memory = 4096_usize
        .checked_mul(std::mem::size_of::<PathBuf>())
        .ok_or_else(|| shell::limit("allocation"))?;
    io.reserve_owned(memory)?;
    let mut paths = Vec::new();
    paths
        .try_reserve_exact(4096)
        .map_err(|_| shell::limit("allocation"))?;
    let mut entries = shell::read_dir(directory, io)?;
    let mut total = 0_u64;
    while let Some(entry) = shell::next(&mut entries, directory, io)? {
        if paths.len() >= 4096 {
            return Err(shell::limit("directory events"));
        }
        let path = shell::entry_path(entry, directory, io)?;
        let (body, metadata) = read(&path, io)?;
        total = total
            .checked_add(body)
            .and_then(|value| value.checked_add(metadata))
            .ok_or_else(|| shell::limit("reservation byte accounting"))?;
        paths.push(path);
    }
    shell::release_directory(directory, io)?;
    Ok((total, paths))
}

pub(super) fn release(
    paths: Vec<PathBuf>,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    for path in &paths {
        io.release_owned(path.as_os_str().len())?;
    }
    io.release_owned(
        4096_usize
            .checked_mul(std::mem::size_of::<PathBuf>())
            .ok_or_else(|| shell::limit("allocation"))?,
    )?;
    drop(paths);
    Ok(())
}

pub(super) fn recover(directory: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    let (_, paths) = scan(directory, io)?;
    for path in &paths {
        io.path_bytes(path.as_os_str().len())?;
        io.path_bytes(directory.as_os_str().len())?;
        io.path_bytes(directory.as_os_str().len())?;
        io.direct_operation(Op::Remove)?;
        io.direct_operation(Op::Open)?;
        io.direct_operation(Op::Sync)?;
        io.reserve_owned(
            path.as_os_str()
                .len()
                .checked_add(512)
                .ok_or_else(|| shell::limit("diagnostics"))?,
        )?;
    }
    for path in &paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(shell::retained_error(path, source, io)?),
        }
        let file = match fs::File::open(directory) {
            Ok(file) => file,
            Err(source) => return Err(shell::retained_error(directory, source, io)?),
        };
        if let Err(source) = file.sync_all() {
            return Err(shell::retained_error(directory, source, io)?);
        }
        io.release_owned(
            path.as_os_str()
                .len()
                .checked_add(512)
                .ok_or_else(|| shell::limit("diagnostics"))?,
        )?;
    }
    release(paths, io)
}

pub(super) fn write(
    path: &Path,
    bytes: (u64, u64),
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    use std::fmt::Write;
    io.reserve_owned(41)?;
    let mut text = String::new();
    text.try_reserve_exact(41)
        .map_err(|_| shell::limit("allocation"))?;
    write!(&mut text, "{}\n{}", bytes.0, bytes.1)
        .map_err(|_| shell::limit("reservation encoding"))?;
    let mut file = shell::create(path, io)?;
    let result = (|| {
        shell::write(&mut file, path, text.as_bytes(), io)?;
        shell::sync(&file, path, io)?;
        shell::sync_directory(
            path.parent()
                .ok_or_else(|| shell::limit("reservation ancestry"))?,
            io,
        )
    })();
    if let Err(error) = result {
        let cleanup = remove(path, io);
        return super::diagnostic::with_cleanup(path, Err(error), cleanup, io);
    }
    io.release_owned(41)?;
    Ok(())
}

pub(super) fn remove(path: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    shell::remove_file(path, io)?;
    shell::sync_directory(
        path.parent()
            .ok_or_else(|| shell::limit("reservation ancestry"))?,
        io,
    )
}
