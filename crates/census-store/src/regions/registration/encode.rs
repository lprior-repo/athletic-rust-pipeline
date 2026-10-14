use super::super::{Envelope, RegionRegistryError, MAX_REGION_RECORD_BYTES};
use census_domain::model::{write_serialized, CanonicalJsonError};
use std::io;

pub(super) fn envelope(value: &Envelope<'_>) -> Result<Vec<u8>, RegionRegistryError> {
    let mut writer = RecordWriter {
        bytes: Vec::new(),
        exceeded: None,
    };
    writer
        .bytes
        .try_reserve_exact(MAX_REGION_RECORD_BYTES)
        .map_err(|source| CanonicalJsonError::Write(io::Error::other(source)))?;
    let result = write_serialized(&mut writer, value);
    if let Some(bytes) = writer.exceeded {
        return Err(RegionRegistryError::Oversized {
            bytes,
            max: MAX_REGION_RECORD_BYTES,
        });
    }
    result?;
    Ok(writer.bytes)
}

struct RecordWriter {
    bytes: Vec<u8>,
    exceeded: Option<usize>,
}

impl io::Write for RecordWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let total = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("regional record length overflow"))?;
        if total > MAX_REGION_RECORD_BYTES {
            self.exceeded = Some(total);
            return Err(io::Error::other("regional record size ceiling"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
