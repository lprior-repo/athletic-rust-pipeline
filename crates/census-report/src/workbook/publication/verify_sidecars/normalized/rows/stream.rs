use std::fs::File;
use std::io::{BufRead, BufReader, Read, Take};
use std::path::Path;

use csv_core::ReadRecordResult;

use super::super::super::{defect, read};
use super::decoder::{add, Decoder, Record};
use crate::report::{io_error, ReportError, ReportResult};

pub(super) fn csv(
    path: &Path,
    limits: read::Limits,
    row_bytes: usize,
    columns: usize,
    mut visit: impl FnMut(usize, &Record<'_>) -> ReportResult<()>,
) -> ReportResult<usize> {
    let mut reader = reader(path, limits.bytes)?;
    let mut decoder = Decoder::new(row_bytes, columns)?;
    let mut consumed = 0;
    let mut records = 0;
    while next(&mut reader, &mut decoder, path, limits.bytes, &mut consumed)
        .map_err(|error| context(path, records, error))?
    {
        records = next_count(path, records, limits.records)?;
        let record = decoder
            .record()
            .map_err(|error| context(path, records, error))?;
        visit(records, &record)?;
        decoder.reset();
    }
    Ok(records)
}

fn next_count(path: &Path, records: usize, limit: usize) -> ReportResult<usize> {
    let records = add(records, 1)?;
    if records > limit {
        return Err(defect(format!(
            "{} holds more than {limit} records",
            path.display()
        )));
    }
    Ok(records)
}

fn next(
    reader: &mut impl BufRead,
    decoder: &mut Decoder,
    path: &Path,
    limit: u64,
    consumed: &mut u64,
) -> ReportResult<bool> {
    loop {
        let input = reader.fill_buf().map_err(|source| io_error(path, source))?;
        let (result, count) = decoder.step(input)?;
        *consumed = add_bytes(*consumed, count, limit)?;
        reader.consume(count);
        match result {
            ReadRecordResult::Record => return Ok(true),
            ReadRecordResult::End => return Ok(false),
            ReadRecordResult::OutputEndsFull => {
                return Err(defect("exceeds normalized column budget".into()));
            }
            ReadRecordResult::InputEmpty | ReadRecordResult::OutputFull => {}
        }
    }
}

fn add_bytes(consumed: u64, count: usize, limit: u64) -> ReportResult<u64> {
    let count = u64::try_from(count).map_err(|error| defect(error.to_string()))?;
    let consumed = consumed
        .checked_add(count)
        .ok_or_else(|| defect("CSV byte count overflow".into()))?;
    if consumed > limit {
        return Err(defect(format!(
            "grew beyond the {limit} byte readback budget"
        )));
    }
    Ok(consumed)
}

fn reader(path: &Path, limit: u64) -> ReportResult<BufReader<Take<File>>> {
    let maximum = limit
        .checked_add(1)
        .ok_or_else(|| defect("CSV byte limit overflow".into()))?;
    Ok(BufReader::new(open(path, limit)?.take(maximum)))
}

fn open(path: &Path, limit: u64) -> ReportResult<File> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() {
        return Err(defect(format!(
            "normalized CSV is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > limit {
        return Err(defect(format!(
            "normalized CSV declares {} bytes beyond the {limit} byte readback budget: {}",
            metadata.len(),
            path.display()
        )));
    }
    File::open(path).map_err(|source| io_error(path, source))
}

fn context(path: &Path, records: usize, error: ReportError) -> ReportError {
    match error {
        ReportError::Invariant { detail } => defect(format!(
            "{} after {records} records: {detail}",
            path.display()
        )),
        error => error,
    }
}
