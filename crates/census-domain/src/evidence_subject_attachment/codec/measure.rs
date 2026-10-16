use super::*;
use std::io::{self, Write};

pub(in super::super) fn measure(value: &impl Serialize, maximum: usize) -> Result<usize> {
    let mut meter = Meter {
        bytes: 0,
        maximum,
        exceeded: None,
        overflowed: false,
    };
    let encoded = crate::model::write_serialized(&mut meter, value);
    if meter.overflowed {
        return Err(EvidenceSubjectAttachmentError::Overflow);
    }
    if let Some(needed) = meter.exceeded {
        return Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "encoded bytes",
            needed,
            available: maximum,
        });
    }
    encoded?;
    Ok(meter.bytes)
}

struct Meter {
    bytes: usize,
    maximum: usize,
    exceeded: Option<usize>,
    overflowed: bool,
}

impl Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(needed) = self.bytes.checked_add(bytes.len()) else {
            self.overflowed = true;
            return Err(io::Error::other("attachment size overflow"));
        };
        if needed > self.maximum {
            self.exceeded = Some(needed);
            return Err(io::Error::other("attachment byte capacity"));
        }
        self.bytes = needed;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
