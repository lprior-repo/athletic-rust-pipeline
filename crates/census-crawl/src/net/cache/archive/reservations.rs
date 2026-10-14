use super::super::archive_io::{invalid, io_error, present, sync_directory, write_file};
use super::super::{is_valid_hex64, read_cache_file, FetchError};
use std::fs;
use std::path::{Path, PathBuf};

pub(in crate::net::cache) const MAX_RECORD_BYTES: usize = 41;

pub(super) fn record_path(
    archive: &Path,
    body: &str,
    metadata: &str,
) -> Result<PathBuf, FetchError> {
    if !is_valid_hex64(body) || !is_valid_hex64(metadata) {
        return Err(invalid(
            archive,
            "reservation key is not a pair of SHA256 digests",
        ));
    }
    Ok(archive
        .join("reservations")
        .join(format!("{body}-{metadata}")))
}

pub(super) fn read_record(path: &Path) -> Result<(u64, u64), FetchError> {
    if !present(path)? {
        return Err(invalid(path, "reservation record is missing"));
    }
    let encoded = read_cache_file(path, MAX_RECORD_BYTES, None)?
        .ok_or_else(|| invalid(path, "reservation record exceeds its limit or changed"))?;
    let text = std::str::from_utf8(&encoded)
        .map_err(|_| invalid(path, "reservation record is not UTF8"))?;
    parse_record(text).map_err(|detail| invalid(path, detail))
}

pub(in crate::net::cache) fn parse_record(text: &str) -> Result<(u64, u64), &'static str> {
    let (body, metadata) = text
        .split_once('\n')
        .ok_or("reservation record requires exactly two byte counts")?;
    let bytes = (parse_count(body)?, parse_count(metadata)?);
    bytes
        .0
        .checked_add(bytes.1)
        .ok_or("reservation byte accounting overflowed")?;
    Ok(bytes)
}

fn parse_count(text: &str) -> Result<u64, &'static str> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("reservation byte count is not an unsigned decimal integer");
    }
    text.parse()
        .map_err(|_| "reservation byte count exceeds its accounting range")
}

pub(super) fn write_record(path: &Path, bytes: (u64, u64)) -> Result<(), FetchError> {
    write_file(path, format!("{}\n{}", bytes.0, bytes.1).as_bytes())?;
    sync_parent(path)
}

pub(super) fn remove_record(path: &Path) -> Result<(), FetchError> {
    match fs::remove_file(path) {
        Ok(()) => sync_parent(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path, error)),
    }
}

fn sync_parent(path: &Path) -> Result<(), FetchError> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid(path, "reservation has no parent"))?;
    sync_directory(parent)
}

pub(super) fn reservation_bytes(archive: &Path) -> Result<u64, FetchError> {
    let directory = archive.join("reservations");
    let mut total = 0_u64;
    for entry in fs::read_dir(&directory).map_err(|error| io_error(&directory, error))? {
        let path = entry.map_err(|error| io_error(&directory, error))?.path();
        validate_key(&path)?;
        let (body, metadata) = read_record(&path)?;
        total = total
            .checked_add(body)
            .and_then(|sum| sum.checked_add(metadata))
            .ok_or_else(|| invalid(&path, "reservation byte accounting overflowed"))?;
    }
    Ok(total)
}

fn validate_key(path: &Path) -> Result<(), FetchError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid(path, "reservation key is not UTF8"))?;
    let (body, metadata) = name
        .split_once('-')
        .ok_or_else(|| invalid(path, "reservation key requires two SHA256 digests"))?;
    if !is_valid_hex64(body) || !is_valid_hex64(metadata) {
        return Err(invalid(
            path,
            "reservation key is not a pair of SHA256 digests",
        ));
    }
    Ok(())
}

pub(super) fn recover_reservations(archive: &Path) -> Result<(), FetchError> {
    reservation_bytes(archive)?;
    let directory = archive.join("reservations");
    for entry in fs::read_dir(&directory).map_err(|error| io_error(&directory, error))? {
        let path = entry.map_err(|error| io_error(&directory, error))?.path();
        remove_record(&path)?;
    }
    Ok(())
}
