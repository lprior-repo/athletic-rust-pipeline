/// Scan-prefix builders for Fjall prefix iteration.
///
/// A prefix stops where the key's next variable-width segment begins, so an
/// event-scoped prefix ends before the checkpoint segment every page-derived key
/// carries: `put_rankings_page` writes that checkpoint, and the readers resolve
/// the accepted checkpoints from the page markers before counting anything.
use super::keys::COLLECTION_PREFIX;
use crate::domain::identity::{AthleteId, EvidenceDigest};

/// Build a marker or presence prefix for one collection and event.
fn event_prefix(collection: &EvidenceDigest, family: &[u8], event_short: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(
        family
            .len()
            .saturating_add(collection.as_str().len())
            .saturating_add(event_short.len())
            .saturating_add(3),
    );
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(family);
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(event_short.as_bytes());
    key.push(0);
    key
}

/// Build a page marker prefix for every page of every event of a collection.
pub(in crate::store) fn collection_page_prefix(collection: &EvidenceDigest) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"pk\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key
}

/// Build a name ref prefix for scanning by collection + name.
pub(in crate::store) fn name_ref_prefix(collection: &EvidenceDigest, name: &str) -> Vec<u8> {
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
pub(in crate::store) fn athlete_ref_prefix(
    collection: &EvidenceDigest,
    athlete: AthleteId,
) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"ra\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key.extend_from_slice(&athlete.get().to_be_bytes());
    key.push(0);
    key
}

/// Build a source result presence prefix for scanning.
pub(in crate::store) fn source_result_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"rs\0", event_short)
}

/// Build a row position prefix for scanning.
pub(in crate::store) fn row_position_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"rr\0", event_short)
}

/// Build an eligible individual prefix for scanning.
pub(in crate::store) fn eligible_individual_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"ri\0", event_short)
}

/// Build an eligible relay prefix for scanning.
pub(in crate::store) fn eligible_relay_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"rm\0", event_short)
}

/// Build a roster missing prefix for scanning.
pub(in crate::store) fn roster_missing_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"rmr\0", event_short)
}

/// Build a roster present prefix for scanning.
pub(in crate::store) fn roster_present_prefix(
    collection: &EvidenceDigest,
    event_short: &str,
) -> Vec<u8> {
    event_prefix(collection, b"rpz\0", event_short)
}

/// Build an event-athlete prefix for scanning all athletes in a collection.
pub(in crate::store) fn event_athlete_prefix(collection: &EvidenceDigest) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(COLLECTION_PREFIX);
    key.extend_from_slice(b"re\0");
    key.extend_from_slice(collection.as_str().as_bytes());
    key.push(0);
    key
}
