use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::Value;

use super::{
    count_at, decode_digest, length, CorruptionClass, Decode, StoreError, StoreResult, CHUNK_TYPE,
    MAX_CHUNK_BYTES,
};
use crate::journal::chunk_key;
use crate::MAX_JOURNAL_VALUE_BYTES;

pub(super) fn chunked(
    decode: &mut Decode,
    phase: &str,
    locator: &str,
    row_key: &[u8],
    manifest: &Value,
    subject: String,
) -> StoreResult<()> {
    let (Some(chunks), Some(total_bytes)) = (
        count_at(manifest, "chunks"),
        count_at(manifest, "total_bytes"),
    ) else {
        decode.note(
            CorruptionClass::Undecodable,
            subject,
            "the manifest records no chunk count or payload length".to_string(),
        );
        return Ok(());
    };
    let mut subject = subject;
    let validation = validate_chunked_manifest(decode, chunks, total_bytes, &mut subject);
    if !validation {
        return Ok(());
    }
    let Some(collected) = read_chunks(decode, row_key, chunks, total_bytes, &mut subject)? else {
        return Ok(());
    };
    if validate_collected_size(decode, collected.len(), total_bytes, &mut subject) {
        decode_digest::digest_chunked(decode, phase, locator, collected, subject);
    }
    Ok(())
}

fn validate_chunked_manifest(
    decode: &mut Decode,
    chunks: usize,
    total_bytes: usize,
    subject: &mut String,
) -> bool {
    if chunks == 0 {
        decode.note(
            CorruptionClass::Truncated,
            std::mem::take(subject),
            "the manifest records no chunks".to_string(),
        );
        return false;
    }
    let ceiling = chunks.saturating_mul(MAX_CHUNK_BYTES);
    if total_bytes > ceiling {
        decode.note(
            CorruptionClass::Oversized,
            std::mem::take(subject),
            format!(
                "the manifest records a {total_bytes}-byte payload for {chunks} chunks, past \
                 the {ceiling}-byte ceiling those chunks carry"
            ),
        );
        return false;
    }
    if !decode.charge(0, length(total_bytes)) {
        return false;
    }
    true
}

fn read_chunks(
    decode: &mut Decode,
    row_key: &[u8],
    chunks: usize,
    total_bytes: usize,
    subject: &mut String,
) -> StoreResult<Option<Vec<u8>>> {
    let mut collected = Vec::with_capacity(total_bytes);
    for index in 0..chunks {
        let chunk = match chunk_key(row_key, index) {
            Ok(chunk) => chunk,
            Err(source) => {
                decode.note(
                    CorruptionClass::Truncated,
                    std::mem::take(subject),
                    source.to_string(),
                );
                return Ok(None);
            }
        };
        if !decode.charge(1, 0) {
            return Ok(None);
        }
        let Some(raw) = decode
            .store
            .journal
            .get(&chunk)
            .map_err(|source| StoreError::Read { source })?
        else {
            decode.note(
                CorruptionClass::Truncated,
                std::mem::take(subject),
                format!("chunk {index} of {chunks} is missing"),
            );
            return Ok(None);
        };
        if !decode.charge(0, length(raw.len())) {
            return Ok(None);
        }
        let Some(decoded) = read_single_chunk(decode, raw.as_ref(), index, subject) else {
            return Ok(None);
        };
        collected.extend_from_slice(&decoded);
        if collected.len() > total_bytes {
            decode.note(
                CorruptionClass::Oversized,
                std::mem::take(subject),
                format!("the chunks carry more than the {total_bytes} bytes the manifest records"),
            );
            return Ok(None);
        }
    }
    Ok(Some(collected))
}

pub(super) fn read_single_chunk(
    decode: &mut Decode,
    raw: &[u8],
    index: usize,
    subject: &mut String,
) -> Option<Vec<u8>> {
    if raw.len() > MAX_JOURNAL_VALUE_BYTES {
        decode.note(
            CorruptionClass::Oversized,
            std::mem::take(subject),
            format!(
                "chunk {index} is {} bytes, past the {MAX_JOURNAL_VALUE_BYTES}-byte physical \
                 value ceiling",
                raw.len()
            ),
        );
        return None;
    }
    let envelope: Value = serde_json::from_slice(raw)
        .inspect_err(|source| {
            decode.note(
                CorruptionClass::Undecodable,
                std::mem::take(subject),
                format!("chunk {index} envelope: {source}"),
            );
        })
        .ok()?;
    if envelope.get("type").and_then(Value::as_str) != Some(CHUNK_TYPE) {
        decode.note(
            CorruptionClass::Undecodable,
            std::mem::take(subject),
            format!("chunk {index} is not a chunk envelope"),
        );
        return None;
    }
    let Some(data) = envelope.get("data").and_then(Value::as_str) else {
        decode.note(
            CorruptionClass::Undecodable,
            std::mem::take(subject),
            format!("chunk {index} carries no data string"),
        );
        return None;
    };
    chunk_bytes(decode, subject, index, data, MAX_CHUNK_BYTES)
}

fn validate_collected_size(
    decode: &mut Decode,
    collected: usize,
    total_bytes: usize,
    subject: &mut String,
) -> bool {
    if collected != total_bytes {
        decode.note(
            CorruptionClass::Truncated,
            std::mem::take(subject),
            format!(
                "the reassembled payload is {} bytes against the {total_bytes} the manifest \
                 records",
                collected
            ),
        );
        return false;
    }
    true
}

pub(super) fn chunk_bytes(
    decode: &mut Decode,
    subject: &str,
    index: usize,
    data: &str,
    budget: usize,
) -> Option<Vec<u8>> {
    let encoded_ceiling = budget.div_ceil(3).saturating_mul(4);
    if data.len() > encoded_ceiling {
        decode.note(
            CorruptionClass::Oversized,
            subject.to_string(),
            format!(
                "chunk {index} carries {} encoded bytes, past the {budget}-byte chunk decode \
                 budget",
                data.len()
            ),
        );
        return None;
    }
    match BASE64_STANDARD.decode(data) {
        Ok(bytes) if bytes.len() > budget => {
            decode.note(
                CorruptionClass::Oversized,
                subject.to_string(),
                format!(
                    "chunk {index} decodes to {} bytes, past the {budget}-byte chunk decode \
                     budget",
                    bytes.len()
                ),
            );
            None
        }
        Ok(bytes) => Some(bytes),
        Err(source) => {
            decode.note(
                CorruptionClass::Undecodable,
                subject.to_string(),
                format!("chunk {index} is not valid base64: {source}"),
            );
            None
        }
    }
}
