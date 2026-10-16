use super::error::invalid;
use super::manifest::{Manifest, CHUNK_BYTES};
use super::{read, RawMetadataArchiveRef, RetainedMetadataError};
use crate::milesplit::RetainedJournalView;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chunk {
    capture: Value,
    source_instance: String,
    role: String,
    chunk_index: usize,
    raw_base64: String,
}

pub(super) fn body(
    view: &impl RetainedJournalView,
    archive: &RawMetadataArchiveRef,
    manifest: &Manifest,
    bytes: usize,
) -> Result<Vec<u8>, RetainedMetadataError> {
    let prefix = archive
        .manifest_key()
        .strip_suffix("/manifest")
        .ok_or(RetainedMetadataError::InvalidArchiveReference)?;
    let mut body = Vec::new();
    body.try_reserve_exact(bytes)?;
    let mut decoded = [0_u8; 32_770];
    for index in 0..manifest.chunks {
        let key = format!("{prefix}/{index}");
        let chunk: Chunk = serde_json::from_value(read(view, &key, 64 * 1024)?)?;
        if chunk.capture != manifest.capture
            || chunk.source_instance != manifest.source_instance
            || chunk.role != manifest.role
            || chunk.chunk_index != index
        {
            return Err(invalid(
                "chunk provenance or index differs from its manifest",
            ));
        }
        let expected = bytes
            .checked_sub(body.len())
            .ok_or_else(|| invalid("chunk exceeds declared capture length"))?
            .min(CHUNK_BYTES);
        let size = decode(&chunk.raw_base64, &mut decoded, expected)?;
        body.extend_from_slice(
            decoded
                .get(..size)
                .ok_or_else(|| invalid("decoded chunk exceeds bounded buffer"))?,
        );
    }
    if body.len() != bytes {
        return Err(invalid(
            "reassembled body differs from declared capture length",
        ));
    }
    Ok(body)
}

fn decode(
    encoded: &str,
    decoded: &mut [u8],
    expected: usize,
) -> Result<usize, RetainedMetadataError> {
    let length = expected
        .checked_add(2)
        .map(|bytes| bytes / 3)
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| invalid("base64 length overflow"))?;
    if encoded.len() != length {
        return Err(invalid("encoded chunk differs from declared body segment"));
    }
    let size = STANDARD
        .decode_slice(encoded, decoded)
        .map_err(|_| invalid("chunk is not canonical base64"))?;
    if size != expected {
        return Err(invalid("decoded chunk differs from declared body segment"));
    }
    Ok(size)
}
