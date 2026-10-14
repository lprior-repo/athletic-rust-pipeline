use super::{CorruptionClass, IntegrityCorruption, Store};

pub(super) const MAX_DECODE_RECORDS: u64 = 25_000;
pub(super) const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MAX_CHUNK_BYTES: usize = 512 * 1024;
pub(super) const MAX_CORRUPTIONS: usize = 1024;
pub(super) const MAX_LIMITS: usize = 16;

pub(super) struct Decode<'a> {
    pub(super) store: &'a Store,
    pub(super) corruptions: Vec<IntegrityCorruption>,
    pub(super) limits: Vec<String>,
    pass: &'static str,
    record_window: u64,
    byte_window: u64,
    records: u64,
    bytes: u64,
    capped: bool,
    window_closed: bool,
}

impl<'a> Decode<'a> {
    pub(super) fn with_window(store: &'a Store, record_window: u64, byte_window: u64) -> Self {
        Self {
            store,
            corruptions: Vec::new(),
            limits: Vec::new(),
            pass: "integrity decode",
            record_window,
            byte_window,
            records: 0,
            bytes: 0,
            capped: false,
            window_closed: false,
        }
    }

    fn open_window(&mut self, pass: &'static str) {
        self.pass = pass;
        self.records = 0;
        self.bytes = 0;
        self.window_closed = false;
    }

    pub(super) fn charge(&mut self, records: u64, bytes: u64) -> bool {
        if self.capped || self.window_closed {
            return false;
        }
        let (Some(next_records), Some(next_bytes)) = (
            self.records.checked_add(records),
            self.bytes.checked_add(bytes),
        ) else {
            self.close_window();
            return false;
        };
        if next_records > self.record_window || next_bytes > self.byte_window {
            self.close_window();
            return false;
        }
        self.records = next_records;
        self.bytes = next_bytes;
        true
    }

    fn close_window(&mut self) {
        self.window_closed = true;
        if self.limits.len() < MAX_LIMITS {
            self.limits.push(format!(
                "{} stopped at the ADR-036 integrity profile window ({} records or {} bytes): \
                 content past it is not decoded",
                self.pass, self.record_window, self.byte_window
            ));
        }
    }

    pub(super) fn note(&mut self, class: CorruptionClass, subject: String, detail: String) -> bool {
        if self.corruptions.len() >= MAX_CORRUPTIONS {
            self.capped = true;
            if self.limits.len() < MAX_LIMITS {
                self.limits.push(format!(
                    "integrity decode stopped after {MAX_CORRUPTIONS} corruption findings"
                ));
            }
            return false;
        }
        self.corruptions.push(IntegrityCorruption {
            class,
            subject,
            detail,
        });
        true
    }

    pub(super) fn set_pass(&mut self, pass: &'static str) {
        self.open_window(pass);
    }
}
