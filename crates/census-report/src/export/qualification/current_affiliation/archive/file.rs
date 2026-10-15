use super::{charge, invalid, MAX_ARCHIVE_BYTES};
use census_store::{StoreError, StoreResult};
use std::io::Read;
use std::path::Path;

pub(super) fn read(
    path: &Path,
    maximum: usize,
    admitted: &mut u64,
) -> StoreResult<Option<Vec<u8>>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io(path, error)),
    };
    if !metadata.is_file()
        || metadata.len() > u64::try_from(maximum).map_err(|_| StoreError::CounterOverflow)?
    {
        return Err(invalid("archive input is not a bounded regular file"));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| StoreError::CounterOverflow)?;
    charge(
        admitted,
        length,
        MAX_ARCHIVE_BYTES,
        "current affiliation archive bytes",
    )?;
    let descriptor = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NOFOLLOW,
        rustix::fs::Mode::empty(),
    )
    .map_err(|source| io(path, source.into()))?;
    let file = std::fs::File::from(descriptor);
    if !file
        .metadata()
        .map_err(|source| io(path, source))?
        .is_file()
    {
        return Err(invalid("archive input changed type during pinned read"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid("archive buffer allocation refused"))?;
    let limit = metadata
        .len()
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|source| io(path, source))?;
    if bytes.len() != length {
        return Err(invalid("archive input changed length during pinned read"));
    }
    Ok(Some(bytes))
}

pub(super) fn directory(path: &Path) -> StoreResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(invalid("archive parent is not a regular directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io(path, error)),
    }
}

fn io(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
