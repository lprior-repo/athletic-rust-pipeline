use crate::report::{io_error, ReportError, ReportResult};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

const MAX_INPUT_BYTES: u64 = super::super::MAX_FROZEN_INPUT_BYTES;

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> ReportResult<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io_error(path, source))?;
    let mut writer = BudgetWriter {
        inner: BufWriter::new(file),
        remaining: MAX_INPUT_BYTES,
    };
    let result = (|| {
        serde_json::to_writer(&mut writer, value).map_err(|source| ReportError::Decode {
            path: path.to_path_buf(),
            line: 0,
            source,
        })?;
        writer.flush().map_err(|source| io_error(path, source))?;
        writer
            .inner
            .get_ref()
            .sync_all()
            .map_err(|source| io_error(path, source))
    })();
    drop(writer);
    if result.is_err() {
        remove_owned(path, result)
    } else {
        result
    }
}

pub(super) fn remove_owned<T>(path: &Path, result: ReportResult<T>) -> ReportResult<T> {
    let cleanup = std::fs::remove_file(path).map_err(|source| io_error(path, source));
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(operation), Err(cleanup)) => Err(ReportError::Cleanup {
            operation: Box::new(operation),
            cleanup: Box::new(cleanup),
        }),
    }
}

pub(super) fn read_json<T: DeserializeOwned>(path: &Path) -> ReportResult<T> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err(ReportError::Invariant {
            detail: format!("invalid or oversized frozen input {}", path.display()),
        });
    }
    let file = std::fs::File::open(path).map_err(|source| io_error(path, source))?;
    serde_json::from_reader(BufReader::new(file)).map_err(|source| ReportError::Decode {
        path: path.to_path_buf(),
        line: 0,
        source,
    })
}

struct BudgetWriter<W> {
    inner: W,
    remaining: u64,
}

impl<W: Write> Write for BudgetWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let requested = u64::try_from(bytes.len()).map_err(std::io::Error::other)?;
        if requested > self.remaining {
            return Err(std::io::Error::other(
                "frozen export input exceeds 8 GiB budget",
            ));
        }
        let written = self.inner.write(bytes)?;
        let consumed = u64::try_from(written).map_err(std::io::Error::other)?;
        self.remaining = self
            .remaining
            .checked_sub(consumed)
            .ok_or_else(|| std::io::Error::other("frozen export input byte counter exhausted"))?;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
