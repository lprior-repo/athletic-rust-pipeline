use super::{limit, refused, MAX_BASE64_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_store::{StoreError, StoreResult};

pub(super) fn copy(bytes: &[u8]) -> StoreResult<Vec<u8>> {
    length(bytes.len())?;
    let mut held = Vec::new();
    held.try_reserve_exact(bytes.len())
        .map_err(|_| refused("MHSAA PDF allocation refused"))?;
    held.extend_from_slice(bytes);
    Ok(held)
}

pub(super) fn encode(bytes: &[u8]) -> StoreResult<String> {
    length(bytes.len())?;
    let maximum = bytes
        .len()
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    limit(maximum, MAX_BASE64_BYTES, "MHSAA encoded PDF bytes")?;
    let mut encoded = String::new();
    encoded
        .try_reserve_exact(maximum)
        .map_err(|_| refused("MHSAA PDF encoding allocation refused"))?;
    STANDARD.encode_string(bytes, &mut encoded);
    Ok(encoded)
}

pub(super) fn decode(encoded: &str, bytes: usize, digest: &str) -> StoreResult<Vec<u8>> {
    length(bytes)?;
    limit(encoded.len(), MAX_BASE64_BYTES, "MHSAA encoded PDF bytes")?;
    super::reference::valid_digest(digest)?;
    let maximum = encoded
        .len()
        .checked_div(4)
        .and_then(|n| n.checked_mul(3))
        .and_then(|n| n.checked_add(3))
        .ok_or(StoreError::CounterOverflow)?;
    let mut original = Vec::new();
    original
        .try_reserve_exact(maximum)
        .map_err(|_| refused("MHSAA PDF decoding allocation refused"))?;
    STANDARD
        .decode_vec(encoded, &mut original)
        .map_err(|_| refused("MHSAA PDF base64 is invalid"))?;
    if original.len() != bytes
        || super::super::hash(&original) != digest
        || encode(&original)? != encoded
    {
        return Err(refused(
            "MHSAA PDF decoded length, hash or canonical encoding differs",
        ));
    }
    Ok(original)
}

pub(super) fn length(bytes: usize) -> StoreResult<()> {
    limit(
        bytes,
        super::super::MAX_PDF_BYTES,
        "MHSAA co-op original PDF bytes",
    )?;
    if bytes == 0 {
        return Err(refused("MHSAA original PDF is empty"));
    }
    Ok(())
}
