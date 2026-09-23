use super::common::{
    validate_event_short, COLLECTION_PREFIX, DIGEST_BYTES, PRESENCE_ELIGIBLE_INDIVIDUAL,
    PRESENCE_ELIGIBLE_RELAY_MEMBER, PRESENCE_EVENT_ATHLETE, PRESENCE_ROSTER_MISSING,
    PRESENCE_ROSTER_PRESENT, PRESENCE_ROW_POSITION, PRESENCE_SOURCE_RESULT, SEAL_KEY,
};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::store::StoreError;

/// Bytes every page-derived presence key spends on the checkpoint segment and
/// its trailing separator.
const CHECKPOINT_SEGMENT_BYTES: usize = DIGEST_BYTES.saturating_add(1);

/// Build presence key for an event-level athlete: rk\0re\0<collection>\0<event>\0<checkpoint>\0<athlete(8B BE)>.
pub(in crate::store) fn presence_event_athlete(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    athlete: AthleteId,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_EVENT_ATHLETE.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_EVENT_ATHLETE);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&athlete.get().to_be_bytes());
    Ok(key)
}

/// Build presence key for an event source result: rk\0rs\0<collection>\0<event>\0<checkpoint>\0<result(8B BE)>.
pub(in crate::store) fn presence_source_result(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_SOURCE_RESULT.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_SOURCE_RESULT);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    Ok(key)
}

/// Build presence key for an event row position: rk\0rr\0<collection>\0<event>\0<checkpoint>\0<result(8B)>\0<position(8B)>.
pub(in crate::store) fn presence_row_position(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
    row_number: u64,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_ROW_POSITION.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8)
        .saturating_add(1)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_ROW_POSITION);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(&row_number.to_be_bytes());
    Ok(key)
}

/// Build presence key for eligible individual: rk\0ri\0<collection>\0<event>\0<checkpoint>\0<result(8B)>\0<athlete(8B)>.
pub(in crate::store) fn presence_eligible_individual(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
    athlete_id: AthleteId,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_ELIGIBLE_INDIVIDUAL.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8)
        .saturating_add(1)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_ELIGIBLE_INDIVIDUAL);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(&athlete_id.get().to_be_bytes());
    Ok(key)
}

/// Build presence key for eligible relay member: rk\0rm\0<collection>\0<event>\0<checkpoint>\0<result(8B)>\0<athlete(8B)>.
pub(in crate::store) fn presence_eligible_relay_member(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
    athlete_id: AthleteId,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_ELIGIBLE_RELAY_MEMBER.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8)
        .saturating_add(1)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_ELIGIBLE_RELAY_MEMBER);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    key.push(b'\0');
    key.extend_from_slice(&athlete_id.get().to_be_bytes());
    Ok(key)
}

/// Build presence key for event roster missing: rk\0rmr\0<collection>\0<event>\0<checkpoint>\0<result(8B)>.
pub(in crate::store) fn presence_roster_missing(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_ROSTER_MISSING.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_ROSTER_MISSING);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    Ok(key)
}

/// Build presence key for event roster present: rk\0rpz\0<collection>\0<event>\0<checkpoint>\0<result(8B)>.
pub(in crate::store) fn presence_roster_present(
    collection: &EvidenceDigest,
    event_short: &str,
    checkpoint: &EvidenceDigest,
    result_id: u64,
) -> Result<Vec<u8>, StoreError> {
    validate_event_short(event_short)?;
    let capacity = COLLECTION_PREFIX
        .len()
        .saturating_add(PRESENCE_ROSTER_PRESENT.len())
        .saturating_add(1)
        .saturating_add(DIGEST_BYTES)
        .saturating_add(1)
        .saturating_add(event_short.len())
        .saturating_add(CHECKPOINT_SEGMENT_BYTES)
        .saturating_add(8);
    let mut key = Vec::with_capacity(capacity);
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(PRESENCE_ROSTER_PRESENT);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(event_short.as_bytes());
    key.push(b'\0');
    key.extend_from_slice(checkpoint.as_str().as_bytes());
    key.push(b'\0');
    key.extend_from_slice(&result_id.to_be_bytes());
    Ok(key)
}

/// Build seal key: rk\0sl\0<collection>.
pub(in crate::store) fn seal_key(collection: &EvidenceDigest) -> Vec<u8> {
    let mut key = Vec::with_capacity(
        COLLECTION_PREFIX
            .len()
            .saturating_add(SEAL_KEY.len())
            .saturating_add(1)
            .saturating_add(DIGEST_BYTES),
    );
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(SEAL_KEY);
    key.extend_from_slice(collection.as_str().as_bytes());
    key
}
