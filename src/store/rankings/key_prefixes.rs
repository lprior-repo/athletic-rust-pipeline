/// Scan-prefix builders for Fjall prefix iteration.
use super::keys::COLLECTION_PREFIX;
use crate::domain::identity::{AthleteId, EvidenceDigest};

/// Build a page marker prefix for scanning all pages of a collection.
pub(super) fn page_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"pk\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build a name ref prefix for scanning by collection + name.
pub(super) fn name_ref_prefix(collection: &EvidenceDigest, name: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rn\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(name.as_bytes());
    key.push(0);
    key
}

/// Build an athlete ref prefix for scanning by collection + athlete.
pub(super) fn athlete_ref_prefix(collection: &EvidenceDigest, athlete: AthleteId) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"ra\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(&athlete.get().to_be_bytes());
    key.push(0);
    key
}

/// Build a source result prefix for scanning.
pub(super) fn source_result_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rs\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build a row position prefix for scanning.
pub(super) fn row_position_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rr\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build an eligible individual prefix for scanning.
pub(super) fn eligible_individual_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"ri\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build an eligible relay prefix for scanning.
pub(super) fn eligible_relay_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rm\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build a roster missing prefix for scanning.
pub(super) fn roster_missing_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rmr\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build a roster present prefix for scanning.
pub(super) fn roster_present_prefix(collection: &EvidenceDigest, event_short: &str) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"rpz\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build an event-athlete prefix for scanning all athletes in a collection.
pub(super) fn event_athlete_prefix(collection: &EvidenceDigest) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"re\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key
}
