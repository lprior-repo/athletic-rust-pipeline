use crate::domain::identity::EvidenceDigest;
use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub fn fingerprint(value: &impl Serialize) -> Result<EvidenceDigest> {
    let mut writer = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(EvidenceDigest::parse(&format!(
        "{:x}",
        writer.0.finalize()
    ))?)
}

pub fn scoped_key(scope: &EvidenceDigest, value: &impl Serialize) -> Result<String> {
    Ok(format!(
        "{}:{}",
        scope.as_str(),
        fingerprint(value)?.as_str()
    ))
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
