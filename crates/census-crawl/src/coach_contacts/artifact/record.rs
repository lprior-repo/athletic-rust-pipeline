use super::io::{invalid, record_buffer};
use super::VERIFIED_CONTACT_HEADER_COUNT;
use csv_core::WriteResult;
use serde::Serialize;
use std::io::{self, Write};

pub(super) struct RecordBuffer {
    bytes: Vec<u8>,
    used: usize,
    csv: csv_core::Writer,
}

impl RecordBuffer {
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self { bytes: record_buffer()?, used: 0, csv: csv_core::Writer::new() })
    }

    pub(super) fn bytes(&self) -> io::Result<&[u8]> {
        self.bytes.get(..self.used).ok_or_else(|| invalid("invalid record length"))
    }

    pub(super) fn json(&mut self, value: &impl Serialize) -> Result<(), serde_json::Error> {
        self.used = 0;
        serde_json::to_writer(self, value)
    }

    pub(super) fn csv(&mut self, fields: [&str; VERIFIED_CONTACT_HEADER_COUNT]) -> io::Result<()> {
        self.used = 0;
        for (index, field) in fields.into_iter().enumerate() {
            if index > 0 {
                let output = self.bytes.get_mut(self.used..).ok_or_else(|| invalid("invalid record length"))?;
                let (result, count) = self.csv.delimiter(output);
                self.advance(result, count)?;
            }
            let output = self.bytes.get_mut(self.used..).ok_or_else(|| invalid("invalid record length"))?;
            let (result, consumed, count) = self.csv.field(field.as_bytes(), output);
            self.advance(result, count)?;
            if consumed != field.len() {
                return Err(invalid("CSV field was not fully encoded"));
            }
        }
        let output = self.bytes.get_mut(self.used..).ok_or_else(|| invalid("invalid record length"))?;
        let (result, count) = self.csv.terminator(output);
        self.advance(result, count)
    }

    fn advance(&mut self, result: WriteResult, count: usize) -> io::Result<()> {
        if result == WriteResult::OutputFull {
            return Err(invalid("record exceeds 1 MiB"));
        }
        self.used = self.used.checked_add(count).filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| invalid("record exceeds 1 MiB"))?;
        Ok(())
    }
}

impl Write for RecordBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self.used.checked_add(bytes.len()).ok_or_else(|| invalid("record length overflow"))?;
        let output = self.bytes.get_mut(self.used..end).ok_or_else(|| invalid("record exceeds 1 MiB"))?;
        output.copy_from_slice(bytes);
        self.used = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
