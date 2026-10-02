use std::io::Write;

use census_store::StoreResult;

use super::Audit;
use crate::consensus::invariant;

pub(super) const MAX_AUDIT_BYTES: usize = 4_194_304;

#[derive(Default)]
struct AuditWriter {
    bytes: Vec<u8>,
}

impl Write for AuditWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("review audit length overflow"))?;
        if length > MAX_AUDIT_BYTES {
            return Err(std::io::Error::other("review audit exceeds 4194304 bytes"));
        }
        if length > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(256)
                .max(length)
                .min(MAX_AUDIT_BYTES);
            self.bytes
                .try_reserve_exact(capacity.saturating_sub(self.bytes.len()))
                .map_err(|error| {
                    std::io::Error::other(format!("cannot reserve review audit: {error}"))
                })?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode(audit: &Audit) -> StoreResult<String> {
    let mut writer = AuditWriter::default();
    serde_json::to_writer(&mut writer, audit)
        .map_err(|error| invariant(format!("cannot retain bounded review advice: {error}")))?;
    String::from_utf8(writer.bytes)
        .map_err(|error| invariant(format!("review audit encoding is not UTF-8: {error}")))
}
