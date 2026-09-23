use super::common::{
    validate_event_short, ATHLETE_REF, COLLECTION_PREFIX, DIGEST_BYTES, MARKER_HASH_BYTES,
    MAX_NAME_BYTES, NAME_REF, PAGE_MARKER, REVOKED_CAPTURE,
};
use super::types::{RankingCandidateKind, RankingRecordRef};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::store::StoreError;

/// Build page marker key: rk\0pk\0<collection>\0<event>\0<page(4B BE)>.
pub(in crate::store) fn page_marker_key(
    collection: &EvidenceDigest,
    event_short: &str,
    page: u32,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    if page == 0 {
        return Err(StoreError::InvalidRankingInput);
    }
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PAGE_MARKER.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(1)
        .saturating_add(4);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PAGE_MARKER);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&page.to_be_bytes());
    Ok(key)
}

/// Build page marker value: `<checkpoint(64 ascii)><index hash(32)>`.
///
/// The checkpoint is the page identity the marker accepts. It is written in the
/// same batch as the page index keys that carry it, so a reader that resolves
/// the marker resolves exactly that capture's slice of the index.
pub(in crate::store) fn page_marker_value(
    checkpoint: &EvidenceDigest,
    index_hash: &[u8],
) -> Result<Vec<u8>, StoreError> {
    if index_hash.len() != MARKER_HASH_BYTES {
        return Err(StoreError::CorruptData);
    }
    let mut value = Vec::with_capacity(DIGEST_BYTES.saturating_add(MARKER_HASH_BYTES));
    value.extend_from_slice(checkpoint.as_str().as_bytes());
    value.extend_from_slice(index_hash);
    Ok(value)
}

/// Parse a page marker value into the accepted checkpoint and the index hash.
pub(in crate::store) fn parse_page_marker_value(
    value: &[u8],
) -> Result<(EvidenceDigest, &[u8]), StoreError> {
    if value.len() != DIGEST_BYTES.saturating_add(MARKER_HASH_BYTES) {
        return Err(StoreError::CorruptData);
    }
    let checkpoint_bytes = value.get(..DIGEST_BYTES).ok_or(StoreError::CorruptData)?;
    let checkpoint_text =
        std::str::from_utf8(checkpoint_bytes).map_err(|_| StoreError::CorruptData)?;
    let checkpoint = EvidenceDigest::parse(checkpoint_text).map_err(|_| StoreError::CorruptData)?;
    let index_hash = value.get(DIGEST_BYTES..).ok_or(StoreError::CorruptData)?;
    Ok((checkpoint, index_hash))
}

/// Build revoked capture key: rk\0rv\0<collection>\0<checkpoint(64B ascii)>.
///
/// Dropping a page marker cannot forget the capture it certified: the abandoned
/// capture's index keys stay stored, keyed by its checkpoint. This entry
/// remembers the index content that capture had, which is the only difference
/// between an identical republication (identical keys, harmless) and a reuse of
/// one capture identity for other content, which the store refuses instead of
/// counting two captures under one accepted checkpoint.
pub(in crate::store) fn revoked_capture_key(
    collection: &EvidenceDigest,
    checkpoint: &EvidenceDigest,
) -> Result<Vec<u8>, StoreError> {
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(REVOKED_CAPTURE.len())
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(REVOKED_CAPTURE);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    Ok(key)
}

/// Build name ref key: rk\0rn\0<collection>\0<name>\0<athlete(8B BE)>\0<checkpoint(64B)>\0<kind(1B)>\0<record_index(4B)>.
///
/// The athlete+checkpoint+kind+record_index suffix ensures no collisions
/// when the same athlete name appears in different checkpoints or kinds.
pub(in crate::store) fn name_ref_key(
    collection: &EvidenceDigest,
    name: &str,
    athlete_id: AthleteId,
    checkpoint: &EvidenceDigest,
    kind: RankingCandidateKind,
    record_index: u32,
) -> Result<Vec<u8>, StoreError> {
    let name_bytes = name.as_bytes();
    if name_bytes.len() > MAX_NAME_BYTES || name_bytes.is_empty() {
        return Err(StoreError::InvalidRankingInput);
    }
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(NAME_REF.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(name_bytes.len())
        .saturating_add(1)
        .saturating_add(8)
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(1)
        .saturating_add(4);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(NAME_REF);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(name_bytes);
    key.push(b'\0');
    key.extend_from_slice(&athlete_id.get().to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    // kind byte: Individual=0, RelayMember=1
    let kind_byte: u8 = match kind {
        RankingCandidateKind::Individual => 0,
        RankingCandidateKind::RelayMember => 1,
    };
    key.push(kind_byte);
    key.extend_from_slice(&record_index.to_be_bytes());
    Ok(key)
}

/// Build athlete ref key: rk\0ra\0<collection>\0<athlete(8B BE)>\0<checkpoint>\0<kind(1B)><record_index(4B)>.
pub(in crate::store) fn athlete_ref_key(
    collection: &EvidenceDigest,
    ref_entry: &RankingRecordRef,
) -> Result<Vec<u8>, StoreError> {
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(ATHLETE_REF.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(8)
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(1)
        .saturating_add(4);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(ATHLETE_REF);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&ref_entry.athlete_id.get().to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(ref_entry.checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.push(match ref_entry.kind {
        RankingCandidateKind::Individual => 0,
        RankingCandidateKind::RelayMember => 1,
    });
    key.extend_from_slice(&ref_entry.record_index.to_be_bytes());
    Ok(key)
}
