use csv_core::{ReadRecordResult, Reader};

use super::super::super::defect;
use crate::report::ReportResult;

pub(super) struct Record<'a> {
    text: &'a str,
    ends: &'a [usize],
}

impl Record<'_> {
    pub(super) fn get(&self, column: usize) -> Option<&str> {
        let start = match column.checked_sub(1) {
            Some(previous) => *self.ends.get(previous)?,
            None => 0,
        };
        self.text.get(start..*self.ends.get(column)?)
    }
}

pub(super) struct Decoder {
    parser: Reader,
    bytes: Vec<u8>,
    ends: Vec<usize>,
    written: usize,
    fields: usize,
    byte_limit: usize,
    columns: usize,
}

impl Decoder {
    pub(super) fn new(byte_limit: usize, columns: usize) -> ReportResult<Self> {
        let capacity = increment(byte_limit)?.min(4096);
        Ok(Self {
            parser: Reader::new(),
            bytes: buffer(capacity)?,
            ends: buffer(increment(columns)?)?,
            written: 0,
            fields: 0,
            byte_limit,
            columns,
        })
    }

    pub(super) fn reset(&mut self) {
        self.written = 0;
        self.fields = 0;
    }

    pub(super) fn step(&mut self, input: &[u8]) -> ReportResult<(ReadRecordResult, usize)> {
        let output = self
            .bytes
            .get_mut(self.written..)
            .ok_or_else(invalid_buffer)?;
        let ends = self
            .ends
            .get_mut(self.fields..)
            .ok_or_else(invalid_buffer)?;
        let (result, consumed, written, fields) = self.parser.read_record(input, output, ends);
        self.written = add(self.written, written)?;
        self.fields = add(self.fields, fields)?;
        if self.written > self.byte_limit {
            return Err(defect("exceeds normalized row byte budget".into()));
        }
        if self.fields > self.columns {
            return Err(defect("exceeds normalized column budget".into()));
        }
        if result == ReadRecordResult::OutputFull {
            self.grow()?;
        }
        Ok((result, consumed))
    }

    fn grow(&mut self) -> ReportResult<()> {
        let maximum = increment(self.byte_limit)?;
        let next = self
            .bytes
            .len()
            .checked_mul(2)
            .ok_or_else(invalid_buffer)?
            .min(maximum);
        if next <= self.bytes.len() {
            return Err(defect("exceeds normalized row byte budget".into()));
        }
        let additional = next
            .checked_sub(self.bytes.len())
            .ok_or_else(invalid_buffer)?;
        self.bytes
            .try_reserve_exact(additional)
            .map_err(allocation)?;
        self.bytes.resize(next, 0);
        Ok(())
    }

    pub(super) fn record(&self) -> ReportResult<Record<'_>> {
        if self.fields != self.columns {
            return Err(defect(format!(
                "holds {} columns; expected {}",
                self.fields, self.columns
            )));
        }
        let bytes = self.bytes.get(..self.written).ok_or_else(invalid_buffer)?;
        let text = std::str::from_utf8(bytes).map_err(|error| defect(error.to_string()))?;
        let ends = self.ends.get(..self.fields).ok_or_else(invalid_buffer)?;
        if ends.iter().any(|end| !text.is_char_boundary(*end)) {
            return Err(defect("non-UTF-8 normalized field".into()));
        }
        Ok(Record { text, ends })
    }
}

fn buffer<T: Default + Clone>(length: usize) -> ReportResult<Vec<T>> {
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(length).map_err(allocation)?;
    buffer.resize(length, T::default());
    Ok(buffer)
}

fn allocation(error: std::collections::TryReserveError) -> crate::report::ReportError {
    defect(format!("normalized CSV buffer allocation: {error}"))
}

fn invalid_buffer() -> crate::report::ReportError {
    defect("invalid normalized CSV buffer".into())
}

pub(super) fn add(left: usize, right: usize) -> ReportResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| defect("normalized CSV counter overflow".into()))
}

fn increment(value: usize) -> ReportResult<usize> {
    add(value, 1)
}
