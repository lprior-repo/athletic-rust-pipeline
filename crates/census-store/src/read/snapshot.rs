use std::io::{BufRead, BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use super::super::{StoreError, StoreResult};
use tracing::debug;

mod temporaries;

pub(crate) use temporaries::sweep_stale_temporaries;
use temporaries::temporary_path;

pub fn write_snapshot_rows<T: Serialize>(path: &Path, rows: &[T]) -> StoreResult<()> {
    publish_atomically(path, |temporary| write_rows(temporary, path, rows))
}

pub fn publish_atomically(
    path: &Path,
    body: impl FnOnce(&Path) -> StoreResult<()>,
) -> StoreResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| io_failure(parent, source))?;
    }
    let temporary = temporary_path(path);
    let staged = body(&temporary).and_then(|()| sync_file(&temporary, path));
    if let Err(failure) = staged {
        discard_temporary(&temporary);
        return Err(failure);
    }
    match std::fs::rename(&temporary, path) {
        Ok(()) => sync_parent_directory(path),
        Err(source) => {
            discard_temporary(&temporary);
            Err(io_failure(path, source))
        }
    }
}

fn write_rows<T: Serialize>(temporary: &Path, published: &Path, rows: &[T]) -> StoreResult<()> {
    let mut writer = open_snapshot_writer(temporary, published)?;
    for row in rows {
        writer.push(row)?;
    }
    writer.finish()
}

pub(crate) fn open_snapshot_writer<'a>(
    temporary: &Path,
    published: &'a Path,
) -> StoreResult<SnapshotRowWriter<'a>> {
    let file = std::fs::File::create(temporary).map_err(|source| io_failure(published, source))?;
    Ok(SnapshotRowWriter {
        writer: BufWriter::new(file),
        published,
    })
}

pub(crate) struct SnapshotRowWriter<'a> {
    writer: BufWriter<std::fs::File>,
    published: &'a Path,
}

impl SnapshotRowWriter<'_> {
    pub(crate) fn push<T: Serialize>(&mut self, row: &T) -> StoreResult<()> {
        serde_json::to_writer(&mut self.writer, row)
            .map_err(|source| json_failure(self.published, source))?;
        self.writer
            .write_all(b"\n")
            .map_err(|source| io_failure(self.published, source))
    }

    pub(crate) fn finish(mut self) -> StoreResult<()> {
        self.writer
            .flush()
            .map_err(|source| io_failure(self.published, source))
    }
}

fn sync_file(temporary: &Path, published: &Path) -> StoreResult<()> {
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(temporary)
        .map_err(|source| io_failure(published, source))?;
    handle
        .sync_all()
        .map_err(|source| io_failure(published, source))
}

fn discard_temporary(temporary: &Path) {
    if let Err(cleanup) = std::fs::remove_file(temporary) {
        debug!(
            ?cleanup,
            ?temporary,
            "snapshot temporary could not be removed"
        );
    }
}

fn sync_parent_directory(path: &Path) -> StoreResult<()> {
    let Some(directory) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    let handle = std::fs::File::open(directory).map_err(|source| io_failure(directory, source))?;
    handle
        .sync_all()
        .map_err(|source| io_failure(directory, source))
}

fn io_failure(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn json_failure(path: &Path, source: serde_json::Error) -> StoreError {
    StoreError::Json {
        detail: format!("writing {}", path.display()),
        source,
    }
}

pub fn csv_failure(published: &Path, error: csv::Error) -> StoreError {
    let detail = error.to_string();
    match error.into_kind() {
        csv::ErrorKind::Io(source) => StoreError::Io {
            path: published.to_path_buf(),
            source,
        },
        _ => StoreError::Invariant {
            detail: format!("writing {}: {detail}", published.display()),
        },
    }
}
pub fn read_rows<T: for<'de> serde::Deserialize<'de>>(
    path: &std::path::Path,
) -> StoreResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut rows = Vec::new();
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(row) => rows.push(row),
            Err(source) => {
                return Err(StoreError::SnapshotRow {
                    path: path.to_path_buf(),
                    line: index.saturating_add(1),
                    source,
                });
            }
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests;
