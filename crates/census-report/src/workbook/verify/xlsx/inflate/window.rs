use crate::report::ReportResult;
use crate::workbook::verify::xlsx::{Scope, Sink};

const WINDOW_BYTES: usize = 32_768;
const FLUSH_BYTES: usize = 8_192;

pub(super) struct Output<'a, S: Sink> {
    sink: &'a mut S,
    scope: Scope<'a>,
    pending: Vec<u8>,
    window: Vec<u8>,
    position: usize,
    held: usize,
    pub(super) total: u64,
    limit: u64,
}

impl<'a, S: Sink> Output<'a, S> {
    pub(super) fn new(sink: &'a mut S, limit: u64, scope: Scope<'a>) -> Self {
        Self {
            sink,
            scope,
            pending: Vec::with_capacity(FLUSH_BYTES.saturating_mul(2)),
            window: vec![0; WINDOW_BYTES],
            position: 0,
            held: 0,
            total: 0,
            limit,
        }
    }

    pub(super) fn push(&mut self, value: u8) -> ReportResult<()> {
        self.total = self.total.checked_add(1).ok_or_else(|| {
            self.scope
                .corrupt("the decompressed size does not fit its count")
        })?;
        if self.total > self.limit {
            return Err(self.scope.corrupt(format!(
                "the decompressed member exceeds the {} byte readback budget",
                self.limit
            )));
        }
        self.pending.push(value);
        if let Some(slot) = self.window.get_mut(self.position) {
            *slot = value;
        }
        self.position = self.position.saturating_add(1) % WINDOW_BYTES;
        self.held = self.held.saturating_add(1).min(WINDOW_BYTES);
        if self.pending.len() >= FLUSH_BYTES {
            self.flush()?;
        }
        Ok(())
    }

    pub(super) fn copy(&mut self, distance: usize, length: usize) -> ReportResult<()> {
        if distance == 0 || distance > self.held {
            return Err(self
                .scope
                .corrupt("a distance reaches before the start of the stream"));
        }
        for _ in 0..length {
            let index = self
                .position
                .saturating_add(WINDOW_BYTES)
                .saturating_sub(distance)
                % WINDOW_BYTES;
            let value = self
                .window
                .get(index)
                .copied()
                .ok_or_else(|| self.scope.corrupt("a distance points outside the window"))?;
            self.push(value)?;
        }
        Ok(())
    }

    pub(super) fn finish(&mut self) -> ReportResult<()> {
        if !self.pending.is_empty() {
            self.sink.push(&self.pending)?;
            self.pending.clear();
        }
        Ok(())
    }

    fn flush(&mut self) -> ReportResult<()> {
        if !self.pending.is_empty() {
            self.sink.push(&self.pending)?;
            self.pending.clear();
        }
        Ok(())
    }
}
