//! CSV writing utility.

use anyhow::Result;
use census_store::read::{csv_failure, publish_atomically};
use census_store::{StoreError, StoreResult};
use std::path::Path;

/// Write a CSV file with the given header and rows.
///
/// Published by rename like every other artifact this repository emits: the export is read by the
/// census document, which opens these tables expecting a complete one, and an in-place write that a
/// crash interrupted leaves a short file the next document pass takes for the real thing.
pub fn write_csv(path: &Path, header: &[&str], rows: &[Vec<String>]) -> Result<()> {
    publish_atomically(path, |temporary| write_body(temporary, path, header, rows))?;
    Ok(())
}

/// Encode the table into `temporary`, the file [`publish_atomically`] renames to `published`.
fn write_body(
    temporary: &Path,
    published: &Path,
    header: &[&str],
    rows: &[Vec<String>],
) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_path(temporary)
        .map_err(|error| csv_failure(published, error))?;
    writer
        .write_record(header)
        .map_err(|error| csv_failure(published, error))?;
    for row in rows {
        writer
            .write_record(row)
            .map_err(|error| csv_failure(published, error))?;
    }
    writer.flush().map_err(|source| StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}
