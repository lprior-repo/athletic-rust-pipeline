use super::buffer::{checked_chunk, Output};
use super::{converter_bytes, converter_io, CONVERTER_READ_CHUNK};
use crate::CrawlResult;
use std::io::Read;
use std::path::Path;

pub(super) fn read(path: &Path, limit: usize) -> CrawlResult<Vec<u8>> {
    let mut file = open(path)?;
    let metadata = file.metadata().map_err(|error| converter_io(path, error))?;
    if !metadata.is_file() {
        return Err(converter_io(
            path,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "converter input is not a regular file",
            ),
        ));
    }
    let mut output = Output::new(limit);
    let mut chunk = [0_u8; CONVERTER_READ_CHUNK];
    for _ in 0..=limit {
        let size = output.read_size()?;
        let buffer = chunk
            .get_mut(..size)
            .ok_or_else(|| converter_bytes(limit))?;
        let read = file
            .read(buffer)
            .map_err(|error| converter_io(path, error))?;
        if read == 0 {
            return Ok(output.into_bytes());
        }
        output.append(checked_chunk(&chunk, read)?, path)?;
    }
    Err(converter_bytes(limit))
}

#[cfg(unix)]
fn open(path: &Path) -> CrawlResult<std::fs::File> {
    use rustix::fs::{Mode, OFlags};

    rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(std::fs::File::from)
    .map_err(|error| converter_io(path, error.into()))
}

#[cfg(not(unix))]
fn open(path: &Path) -> CrawlResult<std::fs::File> {
    std::fs::File::open(path).map_err(|error| converter_io(path, error))
}
