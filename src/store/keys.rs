use super::{error::Result, MAX_DOCUMENT_BYTES};
use crate::domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest};
use sha2::{Digest, Sha256};

const DOCUMENT_PREFIX: &[u8] = b"doc\0";
const SOURCE_PREFIX: &[u8] = b"src\0";
const DIGEST_BYTES: usize = 64;

pub fn document_key(digest: &EvidenceDigest) -> Vec<u8> {
    prefixed_digest(DOCUMENT_PREFIX, digest.as_str().as_bytes())
}

pub fn source_prefix(workbook: &WorkbookDigest) -> Vec<u8> {
    prefixed_digest(SOURCE_PREFIX, workbook.as_str().as_bytes())
}

pub fn source_key(workbook: &WorkbookDigest, row: &SourceRowKey) -> Result<Vec<u8>> {
    let sheet = row.sheet().as_bytes();
    let capacity = SOURCE_PREFIX
        .len()
        .checked_add(DIGEST_BYTES)
        .and_then(|value| value.checked_add(1))
        .and_then(|value| value.checked_add(sheet.len()))
        .and_then(|value| value.checked_add(4))
        .ok_or(super::error::StoreError::BatchTooLarge)?;
    let mut key = Vec::new();
    key.try_reserve(capacity)
        .map_err(|_| super::error::StoreError::BatchTooLarge)?;
    key.extend_from_slice(&source_prefix(workbook));
    key.extend_from_slice(sheet);
    key.push(0);
    key.extend_from_slice(&row.row().to_be_bytes());
    Ok(key)
}

pub fn decode_source_key(key: &[u8]) -> Option<(String, u32)> {
    let prefix_len = SOURCE_PREFIX.len().checked_add(DIGEST_BYTES)?;
    let suffix = key.get(prefix_len..)?;
    let separator = suffix.iter().position(|byte| *byte == 0)?;
    let sheet = std::str::from_utf8(suffix.get(..separator)?)
        .ok()?
        .to_owned();
    let row_bytes = suffix.get(separator.checked_add(1)?..)?;
    let row_array: [u8; 4] = row_bytes.try_into().ok()?;
    Some((sheet, u32::from_be_bytes(row_array)))
}

pub fn valid_document_size(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        Err(super::error::StoreError::DocumentTooLarge)
    } else {
        Ok(())
    }
}

pub fn digest_for(bytes: &[u8]) -> Result<EvidenceDigest> {
    let digest = Sha256::digest(bytes);
    let text = digest
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            text.push(hex_digit(byte >> 4));
            text.push(hex_digit(byte & 0x0f));
            text
        });
    EvidenceDigest::parse(&text).map_err(|_| super::error::StoreError::DigestMismatch)
}

pub fn verify_digest(digest: &EvidenceDigest, bytes: &[u8]) -> Result<()> {
    let actual = digest_for(bytes)?;
    (actual == *digest)
        .then_some(())
        .ok_or(super::error::StoreError::DigestMismatch)
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => '0',
    }
}

fn prefixed_digest(prefix: &[u8], digest: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(prefix.len() + digest.len());
    key.extend_from_slice(prefix);
    key.extend_from_slice(digest);
    key
}
