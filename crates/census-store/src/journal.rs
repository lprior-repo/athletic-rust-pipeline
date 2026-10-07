use base64::prelude::BASE64_STANDARD;
use base64::Engine;

use super::error::StoreError;

pub(super) const JOURNAL_CHUNK_VERSION: u8 = 1;

pub(super) const MANIFEST_TYPE: &str = "chunked-manifest";
pub(super) const CHUNK_TYPE: &str = "chunk";

pub(super) const JOURNAL_CHUNK_RAW: usize = 500_000;

pub(super) fn encode_chunk(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

pub(super) fn decode_chunk(s: &str) -> Result<Vec<u8>, StoreError> {
    BASE64_STANDARD
        .decode(s)
        .map_err(|source| StoreError::Invariant {
            detail: format!("journal chunk is not valid base64: {source}"),
        })
}

pub(super) fn split_chunks(payload: &[u8]) -> Vec<Vec<u8>> {
    if payload.is_empty() {
        return vec![vec![]];
    }
    payload
        .chunks(JOURNAL_CHUNK_RAW)
        .map(|c| c.to_vec())
        .collect()
}

pub(super) fn reassemble_chunks(chunks: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    let mut out = Vec::with_capacity(total);
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
    out
}

pub(super) fn chunk_key(logical_key: &[u8], index: usize) -> Result<Vec<u8>, StoreError> {
    let idx = u32::try_from(index).map_err(|source| StoreError::Refused {
        detail: format!("journal chunk index {index} cannot be encoded: {source}"),
    })?;
    let mut out = logical_key.to_vec();
    out.push(1);
    out.extend_from_slice(&idx.to_be_bytes());
    Ok(out)
}

pub(super) fn parse_chunk_key(raw: &[u8]) -> Option<(&[u8], usize)> {
    let separator = raw.iter().position(|&b| b == 1)?;
    let logical = raw.get(..separator)?;
    let start = separator.checked_add(1)?;
    let end = separator.checked_add(5)?;
    let idx_bytes: [u8; 4] = raw.get(start..end)?.try_into().ok()?;
    let index = usize::try_from(u32::from_be_bytes(idx_bytes)).ok()?;
    Some((logical, index))
}

pub(super) fn manifest_value(
    key: &str,
    chunks: usize,
    total_bytes: usize,
) -> Result<Vec<u8>, StoreError> {
    let manifest = serde_json::json!({
        "type": MANIFEST_TYPE,
        "version": JOURNAL_CHUNK_VERSION,
        "key": key,
        "chunks": chunks,
        "total_bytes": total_bytes,
    });
    serde_json::to_vec(&manifest).map_err(|source| StoreError::Json {
        detail: "serializing journal chunk manifest".to_string(),
        source,
    })
}

pub(super) fn chunk_value(
    key: &str,
    index: usize,
    total: usize,
    data: &[u8],
) -> Result<Vec<u8>, StoreError> {
    let chunk = serde_json::json!({
        "type": CHUNK_TYPE,
        "version": JOURNAL_CHUNK_VERSION,
        "key": key,
        "index": index,
        "total": total,
        "data": encode_chunk(data),
    });
    serde_json::to_vec(&chunk).map_err(|source| StoreError::Json {
        detail: "serializing journal chunk".to_string(),
        source,
    })
}

pub(super) fn is_manifest(raw: &[u8]) -> bool {
    let value: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return false,
    };
    matches!(
        value.get("type").and_then(|t| t.as_str()),
        Some(MANIFEST_TYPE)
    )
}

pub(super) fn manifest_info(raw: &[u8]) -> Result<(String, usize, usize), StoreError> {
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: "journal manifest".to_string(),
            source,
        })?;
    let key = value
        .get("key")
        .and_then(|k| k.as_str())
        .ok_or_else(|| StoreError::Invariant {
            detail: "journal manifest has no key".to_string(),
        })?
        .to_string();
    let chunks = value
        .get("chunks")
        .and_then(|c| c.as_u64())
        .ok_or_else(|| StoreError::Invariant {
            detail: "journal manifest has no chunks count".to_string(),
        })?
        .try_into()
        .map_err(|_| StoreError::CounterOverflow)?;
    let total_bytes = value
        .get("total_bytes")
        .and_then(|b| b.as_u64())
        .ok_or_else(|| StoreError::Invariant {
            detail: "journal manifest has no total_bytes".to_string(),
        })?
        .try_into()
        .map_err(|_| StoreError::CounterOverflow)?;
    Ok((key, chunks, total_bytes))
}
