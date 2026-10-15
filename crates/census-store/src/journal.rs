use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

use super::error::StoreError;

pub(super) const JOURNAL_CHUNK_VERSION: u8 = 1;

pub(super) const MANIFEST_TYPE: &str = "chunked-manifest";
pub(super) const CHUNK_TYPE: &str = "chunk";

pub(super) const JOURNAL_CHUNK_RAW: usize = 500_000;

pub(super) fn encode_chunk(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

pub(super) fn decode_chunk_into(
    data: &str,
    output: &mut [u8],
) -> Result<usize, base64::DecodeSliceError> {
    BASE64_STANDARD.decode_slice(data, output)
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

pub(super) const JOURNAL_OBJECT_VERSION: u8 = 2;

pub(super) const OBJECT_ALIAS_TYPE: &str = "journal-alias";
pub(super) const OBJECT_MANIFEST_TYPE: &str = "journal-manifest";
pub(super) const OBJECT_INDEX_TYPE: &str = "journal-index";
pub(super) const OBJECT_CHUNK_TYPE: &str = "journal-chunk";

pub(super) const OBJECT_CODEC_JSONL: &str = "jsonl";
pub(super) const OBJECT_STATE_READY: &str = "ready";

pub(super) const OBJECT_NAMESPACE: &[u8] = b"j2";
pub(super) const OBJECT_MANIFEST_SEGMENT: &[u8] = b"manifest";
pub(super) const OBJECT_INDEX_SEGMENT: &[u8] = b"index";
pub(super) const OBJECT_CHUNK_SEGMENT: &[u8] = b"chunk";

pub(super) const OBJECT_ID_BYTES: usize = 64;
pub(super) const OBJECT_BYTES_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
pub(super) const OBJECT_CHUNKS_LIMIT: u64 = 8_192;
pub(super) const OBJECT_RECORDS_LIMIT: u64 = 4_194_304;
pub(super) const OBJECT_RECORD_BYTES_LIMIT: usize = 64 * 1024 * 1024;
pub(super) const OBJECT_CHUNK_BYTES: usize = 512 * 1024;
pub(super) const OBJECT_INDEX_PAGE_ENTRIES: usize = 512;

pub(super) fn object_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn decode_object_chunk(data: &str) -> Option<Vec<u8>> {
    BASE64_STANDARD.decode(data).ok()
}

pub(super) fn is_object_id(value: &str) -> bool {
    value.len() == OBJECT_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn object_manifest_key(object: &str) -> Result<Vec<u8>, StoreError> {
    object_key(OBJECT_MANIFEST_SEGMENT, object, None)
}

pub(super) fn object_index_key(object: &str, page: usize) -> Result<Vec<u8>, StoreError> {
    object_key(OBJECT_INDEX_SEGMENT, object, Some(page))
}

pub(super) fn object_chunk_key(object: &str, index: usize) -> Result<Vec<u8>, StoreError> {
    object_key(OBJECT_CHUNK_SEGMENT, object, Some(index))
}

pub(super) fn object_index_prefix(object: &str) -> Result<Vec<u8>, StoreError> {
    object_ordinal_prefix(OBJECT_INDEX_SEGMENT, object)
}

pub(super) fn object_chunk_prefix(object: &str) -> Result<Vec<u8>, StoreError> {
    object_ordinal_prefix(OBJECT_CHUNK_SEGMENT, object)
}

pub(super) fn object_ordinal(raw: &[u8], prefix: &[u8]) -> Option<usize> {
    let suffix = raw.strip_prefix(prefix)?;
    let bytes: [u8; 4] = suffix.try_into().ok()?;
    usize::try_from(u32::from_be_bytes(bytes)).ok()
}

pub(super) fn expected_chunk_count(total_bytes: usize) -> usize {
    if total_bytes == 0 {
        1
    } else {
        total_bytes.div_ceil(OBJECT_CHUNK_BYTES)
    }
}

pub(super) fn expected_chunk_length(
    index: usize,
    chunks: usize,
    total_bytes: usize,
) -> Option<usize> {
    let last = chunks.checked_sub(1)?;
    if index < last {
        return Some(OBJECT_CHUNK_BYTES);
    }
    if index > last {
        return None;
    }
    let head = last.checked_mul(OBJECT_CHUNK_BYTES)?;
    let tail = total_bytes.checked_sub(head)?;
    (tail <= OBJECT_CHUNK_BYTES).then_some(tail)
}

pub(super) fn expected_index_pages(records: usize) -> usize {
    records.div_ceil(OBJECT_INDEX_PAGE_ENTRIES)
}

pub(super) fn expected_index_entries(page: usize, pages: usize, records: usize) -> Option<usize> {
    let last = pages.checked_sub(1)?;
    if page < last {
        return Some(OBJECT_INDEX_PAGE_ENTRIES);
    }
    if page > last {
        return None;
    }
    let head = last.checked_mul(OBJECT_INDEX_PAGE_ENTRIES)?;
    records.checked_sub(head)
}

fn object_ordinal_prefix(segment: &[u8], object: &str) -> Result<Vec<u8>, StoreError> {
    let mut out = object_key(segment, object, None)?;
    out.push(0);
    Ok(out)
}

fn object_key(segment: &[u8], object: &str, ordinal: Option<usize>) -> Result<Vec<u8>, StoreError> {
    let mut out = Vec::with_capacity(
        OBJECT_NAMESPACE
            .len()
            .saturating_add(segment.len())
            .saturating_add(object.len())
            .saturating_add(7),
    );
    out.extend_from_slice(OBJECT_NAMESPACE);
    out.push(0);
    out.extend_from_slice(segment);
    out.push(0);
    out.extend_from_slice(object.as_bytes());
    if let Some(value) = ordinal {
        let encoded = u32::try_from(value).map_err(|source| StoreError::Refused {
            detail: format!("journal object ordinal {value} cannot be encoded: {source}"),
        })?;
        out.push(0);
        out.extend_from_slice(&encoded.to_be_bytes());
    }
    Ok(out)
}
