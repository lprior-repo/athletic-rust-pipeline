use super::super::super::write_progress;
use super::TestResult;
use census_store::StoreError;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

pub(super) struct Writer {
    pub(super) bytes: Vec<u8>,
    pub(super) label: String,
    pub(super) trace: Rc<RefCell<Vec<String>>>,
    pub(super) fail: Option<&'static str>,
    pub(super) short: bool,
}

impl Writer {
    pub(super) fn new(label: &str, trace: Rc<RefCell<Vec<String>>>) -> Self {
        Self {
            bytes: Vec::new(),
            label: label.into(),
            trace,
            fail: None,
            short: false,
        }
    }
}

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let part = if bytes == b"\n" { "lf" } else { "body" };
        self.trace
            .borrow_mut()
            .push(format!("{}:{part}", self.label));
        if self.fail == Some("zero") {
            return Ok(0);
        }
        if self.fail == Some(part) || self.fail == Some("write") {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer sentinel"));
        }
        let bytes = if self.short {
            bytes
                .get(..1)
                .ok_or_else(|| io::Error::other("empty write"))?
        } else {
            bytes
        };
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.trace
            .borrow_mut()
            .push(format!("{}:flush", self.label));
        if self.fail == Some("flush") {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer sentinel"));
        }
        Ok(())
    }
}

pub(super) fn sentinel(error: &anyhow::Error, message: &str) -> TestResult {
    let cause = if let Some(cause) = error.downcast_ref::<io::Error>() {
        cause
    } else {
        let Some(StoreError::Io { source, .. }) = error.downcast_ref::<StoreError>() else {
            return Err(format!("missing I/O cause: {error:#}").into());
        };
        source
    };
    check!(eq; cause.kind(), io::ErrorKind::BrokenPipe);
    check!(eq; cause.to_string(), message);
    Ok(())
}

#[test]
fn production_encoder_handles_short_zero_broken_and_flush_writes() -> TestResult {
    let mut normal = Vec::new();
    write_progress(&mut normal, "snapshot_held")?;
    for failure in [None, Some("zero"), Some("write"), Some("flush")] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let mut output = Writer::new("progress", trace.clone());
        output.short = true;
        output.fail = failure;
        match write_progress(&mut output, "snapshot_held") {
            Ok(()) => {
                check!(failure.is_none());
                check!(eq; output.bytes, normal);
            }
            Err(error) => {
                check!(failure.is_some());
                check!(eq; error.kind(), if failure == Some("zero") { io::ErrorKind::WriteZero } else { io::ErrorKind::BrokenPipe });
                if failure == Some("flush") {
                    check!(eq; output.bytes, normal);
                }
            }
        }
        let flushes = trace
            .borrow()
            .iter()
            .filter(|event| event.ends_with(":flush"))
            .count();
        check!(eq; flushes, usize::from(failure.is_none() || failure == Some("flush")));
    }
    let mut output = Vec::new();
    let error = write_progress(&mut output, "unknown")
        .err()
        .ok_or("unknown stage accepted")?;
    check!(eq; error.kind(), io::ErrorKind::InvalidInput);
    check!(output.is_empty());
    Ok(())
}
