use super::Runtime;
use crate::{domain::identity::EvidenceDigest, store::MAX_DOCUMENT_BYTES};
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, Write};

impl Runtime {
    /// Verified immutable reads may repeat on replay; only references enter journals.
    pub async fn load_json<T>(&self, digest: &EvidenceDigest) -> Result<T>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let store = self.store.clone();
        let digest = digest.clone();
        self.blocking(move || {
            let bytes = store.get_bytes(&digest)?;
            serde_json::from_slice(&bytes).context("decoding verified JSON artifact")
        })
        .await
    }

    /// Call inside a journaled run when publishing a newly derived artifact.
    pub async fn store_json<T>(&self, value: T) -> Result<EvidenceDigest>
    where
        T: Serialize + Send + 'static,
    {
        let store = self.store.clone();
        self.blocking(move || {
            let bytes = encode(&value)?;
            Ok(store.put_bytes(&bytes)?)
        })
        .await
    }
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut output = LimitedJson(Vec::new());
    serde_json::to_writer(&mut output, value).context("encoding bounded JSON artifact")?;
    Ok(output.0)
}

struct LimitedJson(Vec<u8>);

impl Write for LimitedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let size = self
            .0
            .len()
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_DOCUMENT_BYTES)
            .ok_or_else(|| io::Error::other("JSON artifact exceeds document limit"))?;
        self.0
            .try_reserve(size - self.0.len())
            .map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_limit_includes_json_escaping() {
        let input = "\u{0001}".repeat(MAX_DOCUMENT_BYTES / 6 + 1);
        assert!(encode(&input).is_err());
    }
}
