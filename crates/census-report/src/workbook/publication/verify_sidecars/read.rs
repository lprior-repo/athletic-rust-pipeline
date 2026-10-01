use crate::report::{io_error, ReportError, ReportResult};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use super::cells::Cell;
use super::{defect, excerpt};

const MAX_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    pub(super) bytes: u64,
    pub(super) records: usize,
}

pub(super) fn json_value(path: &Path, bytes: u64, what: &str) -> ReportResult<Value> {
    json_as(path, bytes, what)
}

pub(super) fn json_as<T: DeserializeOwned>(path: &Path, bytes: u64, what: &str) -> ReportResult<T> {
    let buffer = read_bytes(path, bytes, what)?;
    serde_json::from_slice(&buffer).map_err(|source| decode_error(path, 0, source))
}

pub(super) fn lines(
    path: &Path,
    limits: Limits,
    what: &str,
    mut visit: impl FnMut(usize, &str) -> ReportResult<()>,
) -> ReportResult<usize> {
    let file = open(path, limits.bytes, what)?;
    let mut reader = BufReader::new(file).take(limits.bytes.saturating_add(1));
    let mut buffer: Vec<u8> = Vec::new();
    let mut line = 0usize;
    let mut records = 0usize;
    while read_line(&mut reader, &mut buffer).map_err(|source| io_error(path, source))? {
        line = line.saturating_add(1);
        let text = std::str::from_utf8(buffer.as_slice()).map_err(|_| {
            defect(format!(
                "{what} holds non-UTF-8 text at line {line}: {}",
                path.display()
            ))
        })?;
        let text = text.trim();
        if text.is_empty() {
            let detail = format!(
                "{what} holds an empty record at line {line}: {}",
                path.display()
            );
            return Err(defect(detail));
        }
        records = records.saturating_add(1);
        if records > limits.records {
            return Err(defect(format!(
                "{what} holds more than {} records: {}",
                limits.records,
                path.display()
            )));
        }
        visit(line, text)?;
    }
    Ok(records)
}

pub(super) fn csv(
    path: &Path,
    limits: Limits,
    what: &str,
    mut visit: impl FnMut(usize, &csv::StringRecord) -> ReportResult<()>,
) -> ReportResult<usize> {
    let file = open(path, limits.bytes, what)?;
    let reader = BufReader::new(file).take(limits.bytes.saturating_add(1));
    let mut parser = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    let mut records = 0usize;
    for record in parser.records() {
        let record = record.map_err(|error| csv_error(path, error))?;
        records = records.saturating_add(1);
        if records > limits.records {
            return Err(defect(format!(
                "{what} holds more than {} records: {}",
                limits.records,
                path.display()
            )));
        }
        visit(records, &record)?;
    }
    Ok(records)
}

pub(super) fn headers(
    path: &Path,
    record: &csv::StringRecord,
    expected: &[&str],
) -> ReportResult<()> {
    if record.len() != expected.len() {
        return Err(defect(format!(
            "{} header holds {} columns where the publication writes {}",
            path.display(),
            record.len(),
            expected.len()
        )));
    }
    for (column, name) in expected.iter().enumerate() {
        let found = record.get(column).unwrap_or_default();
        if found != *name {
            return Err(defect(format!(
                "{} header column {column}: expected {name:?}, found {:?}",
                path.display(),
                excerpt(found)
            )));
        }
    }
    Ok(())
}

pub(super) fn compare_record(
    path: &Path,
    record: usize,
    found: &csv::StringRecord,
    cells: &[Cell],
) -> ReportResult<()> {
    if found.len() != cells.len() {
        return Err(defect(format!(
            "{} record {record} holds {} columns where the shared reduction projects {}",
            path.display(),
            found.len(),
            cells.len()
        )));
    }
    for (column, cell) in cells.iter().enumerate() {
        let value = found.get(column).unwrap_or_default();
        cell.compare(path, record, column, value)?;
    }
    Ok(())
}

fn read_bytes(path: &Path, bytes: u64, what: &str) -> ReportResult<Vec<u8>> {
    let file = open(path, bytes, what)?;
    let mut reader = BufReader::new(file).take(bytes.saturating_add(1));
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .map_err(|source| io_error(path, source))?;
    let length = u64::try_from(buffer.len()).map_err(|error| defect(error.to_string()))?;
    if length > bytes {
        return Err(defect(format!(
            "{what} grew beyond the {bytes} byte readback budget: {}",
            path.display()
        )));
    }
    Ok(buffer)
}

fn open(path: &Path, bytes: u64, what: &str) -> ReportResult<File> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() {
        return Err(defect(format!(
            "{what} is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > bytes {
        return Err(defect(format!(
            "{what} declares {} bytes beyond the {bytes} byte readback budget: {}",
            metadata.len(),
            path.display()
        )));
    }
    File::open(path).map_err(|source| io_error(path, source))
}

fn read_line(reader: &mut impl BufRead, buffer: &mut Vec<u8>) -> std::io::Result<bool> {
    buffer.clear();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(!buffer.is_empty());
        }
        let position = available.iter().position(|byte| *byte == b'\n');
        let length = position.map_or(available.len(), |position| position.saturating_add(1));
        if buffer.len().saturating_add(length) > MAX_RECORD_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "record exceeds the readback line budget",
            ));
        }
        let chunk = available.get(..length).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid readback buffer")
        })?;
        buffer.extend_from_slice(chunk);
        let complete = position.is_some();
        reader.consume(length);
        if complete {
            return Ok(true);
        }
    }
}

fn csv_error(path: &Path, error: csv::Error) -> ReportError {
    io_error(path, std::io::Error::other(error))
}

fn decode_error(path: &Path, line: usize, source: serde_json::Error) -> ReportError {
    ReportError::Decode {
        path: path.to_path_buf(),
        line,
        source,
    }
}
