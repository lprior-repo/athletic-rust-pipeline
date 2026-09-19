use crate::store::StoreError;


pub(in crate::store) const COLLECTION_PREFIX: &[u8] = b"rk\0";
pub(in crate::store) const DIGEST_BYTES: usize = 64;
pub(in crate::store) const MAX_EVENT_SHORT: usize = 128;
pub(in crate::store) const MAX_NAME_BYTES: usize = 256;
pub(in crate::store) const NAME_REF: &[u8] = b"rn\0";
pub(in crate::store) const ATHLETE_REF: &[u8] = b"ra\0";
pub(in crate::store) const PAGE_MARKER: &[u8] = b"pk\0";
pub(in crate::store) const SEAL_KEY: &[u8] = b"sl\0";
pub(in crate::store) const PRESENCE_ROW_POSITION: &[u8] = b"rr\0";
pub(in crate::store) const PRESENCE_ELIGIBLE_INDIVIDUAL: &[u8] = b"ri\0";
pub(in crate::store) const PRESENCE_ELIGIBLE_RELAY_MEMBER: &[u8] = b"rm\0";
pub(in crate::store) const PRESENCE_ROSTER_MISSING: &[u8] = b"rmr\0";
pub(in crate::store) const PRESENCE_ROSTER_PRESENT: &[u8] = b"rpz\0";
pub(in crate::store) const PRESENCE_SOURCE_RESULT: &[u8] = b"rs\0";
pub(in crate::store) const PRESENCE_EVENT_ATHLETE: &[u8] = b"re\0";

/// Validate event_short before key construction.
pub(super) fn validate_event_short(value: &str) -> Result<(), StoreError> {
    if value.is_empty() || value.len() > MAX_EVENT_SHORT {
        return Err(StoreError::InvalidRankingInput);
    }
    if value.as_bytes().iter().any(|b| *b < 0x20 || *b == b'\x7f') {
        return Err(StoreError::InvalidRankingInput);
    }
    Ok(())
}
