use super::error::{corrupt, foreign};
use super::manifests::{same_content, validate_capture, Layout, Occurrence};
use super::{decode, read, CaptureWitnessError, OwnedAcquisitionRef, CHUNK_JOURNAL_BYTES};
use crate::milesplit::owned::effect::CAPTURE_CHUNK_BYTES;
use crate::net::FetchOutcome;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_store::StoreSnapshot;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chunk {
    capture: FetchOutcome,
    chunk_index: usize,
    raw_base64: String,
}

pub(super) fn read_body(
    view: &StoreSnapshot<'_>,
    reference: &OwnedAcquisitionRef,
    occurrence: &Occurrence,
    layout: Layout,
) -> Result<Vec<u8>, CaptureWitnessError> {
    let mut body = Vec::new();
    body.try_reserve_exact(layout.bytes)
        .map_err(|source| CaptureWitnessError::Allocation {
            key: reference.manifest_key().into(),
            needed: layout.bytes,
            source,
        })?;
    let mut decoded = [0_u8; 32_770];
    for index in 0..layout.chunks {
        let key = format!("{}/{index}", occurrence.chunk_key_prefix);
        let chunk: Chunk = decode(read(view, &key, CHUNK_JOURNAL_BYTES)?, &key)?;
        validate_capture(&chunk.capture, reference, &key)?;
        if !same_content(&chunk.capture, &occurrence.capture) {
            return Err(foreign(
                &key,
                "chunk capture metadata has different immutable content ownership",
            ));
        }
        if chunk.chunk_index != index {
            return Err(corrupt(
                &key,
                "source chunk index does not match its requested key",
            ));
        }
        let remaining = layout
            .bytes
            .checked_sub(body.len())
            .ok_or_else(|| corrupt(&key, "reassembled bytes exceed declared body length"))?;
        let expected = remaining.min(CAPTURE_CHUNK_BYTES);
        let size = decode_into(&chunk.raw_base64, &mut decoded, expected, &key)?;
        let bytes = decoded
            .get(..size)
            .ok_or_else(|| corrupt(&key, "decoded chunk exceeds buffer"))?;
        body.extend_from_slice(bytes);
    }
    if body.len() != layout.bytes {
        return Err(corrupt(
            reference.manifest_key(),
            "restored body length differs from retained length",
        ));
    }
    Ok(body)
}

fn decode_into(
    encoded: &str,
    decoded: &mut [u8],
    expected: usize,
    key: &str,
) -> Result<usize, CaptureWitnessError> {
    let length = expected
        .checked_add(2)
        .map(|bytes| bytes / 3)
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| corrupt(key, "source chunk base64 length overflow"))?;
    if encoded.len() != length {
        return Err(corrupt(
            key,
            "source chunk base64 length does not match its declared body segment",
        ));
    }
    let size = STANDARD.decode_slice(encoded, decoded).map_err(|source| {
        corrupt(
            key,
            format!("source chunk is not canonical base64: {source}"),
        )
    })?;
    if size != expected {
        return Err(corrupt(
            key,
            "decoded source chunk length does not match its declared body segment",
        ));
    }
    Ok(size)
}
