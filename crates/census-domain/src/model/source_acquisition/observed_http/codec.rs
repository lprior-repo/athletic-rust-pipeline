use super::{CaptureDomainError, NormalizedObservedHttpV1};
use crate::model::MAX_CAPTURE_RECORD_BYTES;
use std::io::{self, Write};

pub(super) fn ensure_limit(value: &NormalizedObservedHttpV1) -> Result<(), CaptureDomainError> {
    let mut counter = Counter {
        bytes: 0,
        exceeded: false,
    };
    let encoded = serde_json::to_writer(&mut counter, value);
    if counter.exceeded {
        return Err(CaptureDomainError::Bound {
            field: "observed metadata bytes",
            max: MAX_CAPTURE_RECORD_BYTES,
        });
    }
    encoded.map_err(|error| CaptureDomainError::Encoding(error.to_string()))?;
    if counter.bytes == 0 {
        return Err(CaptureDomainError::Invalid {
            field: "observed metadata bytes",
        });
    }
    Ok(())
}

struct Counter {
    bytes: usize,
    exceeded: bool,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.bytes.checked_add(bytes.len()) {
            Some(total) if total <= MAX_CAPTURE_RECORD_BYTES => {
                self.bytes = total;
                Ok(bytes.len())
            }
            Some(_) | None => {
                self.exceeded = true;
                Err(io::Error::other("observed metadata bytes exceed limit"))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
