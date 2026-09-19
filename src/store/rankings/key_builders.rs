use crate::store::StoreError;
use super::common::{validate_event_short, ATHLETE_REF, COLLECTION_PREFIX, DIGEST_BYTES, MAX_NAME_BYTES, NAME_REF, PAGE_MARKER};
use super::types::{RankingCandidateEntry, RankingCandidateKind, RankingRecordRef};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use sha2::{Digest, Sha256};

/// Build page marker key: rk\0pk\0<collection>\0<event>\0<page(8B BE)>.
pub(super) fn page_marker_key(
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
        .saturating_add(8);
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

/// Build name ref key: rk\0rn\0<collection>\0<name>\0<athlete(8B BE)>\0<checkpoint(64B)>\0<kind(1B)>\0<record_index(4B)>.
///
/// The athlete+checkpoint+kind+record_index suffix ensures no collisions
/// when the same athlete name appears in different checkpoints or kinds.
pub(super) fn name_ref_key(
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
    // kind as u8: Individual=0, RelayMember=1
    let kind_byte: u8 = match kind {
        RankingCandidateKind::Individual => 0,
        RankingCandidateKind::RelayMember => 1,
    };
    key.push(kind_byte);
    key.extend_from_slice(&record_index.to_be_bytes());
    Ok(key)
}

/// Build athlete ref key: rk\0ra\0<collection>\0<athlete(8B BE)>\0<checkpoint>\0.
pub(super) fn athlete_ref_key(
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
        .saturating_add(1);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(ATHLETE_REF);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&ref_entry.athlete_id.get().to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(ref_entry.checkpoint.as_str().as_bytes());
    key.push(b'\0');
    Ok(key)
}
