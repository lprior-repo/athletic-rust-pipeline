use super::*;
use std::io::Write;

pub(super) fn encode(
    value: &impl serde::Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let expected = admit(value, grant)?;
    let bytes = census_domain::model::serialized_bytes(value)?;
    if bytes.len() != expected {
        return Err(invalid("roster family inventory changed during encoding"));
    }
    Ok(bytes)
}

struct Counter {
    bytes: usize,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("roster inventory arithmetic overflow"))?;
        if count > 8 * 1024 * 1024 {
            return Err(std::io::Error::other(
                "roster inventory byte ceiling exceeded",
            ));
        }
        self.bytes = count;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn admit(
    value: &impl serde::Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    let mut counter = Counter { bytes: 0 };
    census_domain::model::write_serialized(&mut counter, value)?;
    charge(grant, counter.bytes, 1)?;
    Ok(counter.bytes)
}
