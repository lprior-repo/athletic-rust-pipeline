use crate::report::ReportResult;
use serde::Serialize;
use std::io::{Error, Write};

pub(super) fn require(value: &impl Serialize, expected: &[u8]) -> ReportResult<()> {
    let mut sink = Compare {
        expected,
        offset: 0,
        failed: false,
    };
    let encoded = serde_json::to_writer(&mut sink, value);
    if encoded.is_err() || sink.failed || sink.offset != expected.len() {
        return Err(
            super::super::invalid("participation ReplayV2 payload is not canonical").into(),
        );
    }
    Ok(())
}

struct Compare<'a> {
    expected: &'a [u8],
    offset: usize,
    failed: bool,
}

impl Write for Compare<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let Some(end) = self.offset.checked_add(bytes.len()) else {
            self.failed = true;
            return Err(Error::other("participation canonical length overflow"));
        };
        if self.failed || self.expected.get(self.offset..end) != Some(bytes) {
            self.failed = true;
            return Err(Error::other("participation canonical bytes differ"));
        }
        self.offset = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
