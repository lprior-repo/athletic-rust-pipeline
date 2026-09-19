use super::keys::{
    eligible_individual_prefix, eligible_relay_prefix, roster_missing_prefix,
    roster_present_prefix, row_position_prefix, source_result_prefix, validate_event_short,
    COLLECTION_PREFIX,
};
use super::types::{RankingCollectionStats, RankingEventStats};
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::Readable;

/// Decode a fixed-width big-endian u64 at the given offset from the key.
fn decode_be_u64(key: &[u8], offset: usize) -> Option<u64> {
    let slice = key.get(offset..offset + 8)?;
    let bytes: [u8; 8] = slice.try_into().ok()?;
    Some(u64::from_be_bytes(bytes))
}

/// Compute event-level stats via streaming key-prefix scans.
pub(in crate::store) fn ranking_event_stats(
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
) -> Result<RankingEventStats, StoreError> {
    validate_event_short(event_short)?;

    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    let snap = store.database.snapshot();

    let page_prefix = super::keys::page_prefix(collection, event_short);
    let mut pages = 0u64;
    for guard in snap.prefix(&store.rankings, page_prefix) {
        let _key = guard.key().map_err(|_| StoreError::CorruptData)?;
        pages += 1;
    }

    let source_prefix = source_result_prefix(collection, event_short);
    let mut source_results: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut row_positions: u64 = 0;
    let mut max_row_position: u64 = 0;
    let mut grade11_individual: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut grade11_relay: std::collections::BTreeSet<(u64, u64)> =
        std::collections::BTreeSet::new();
    let mut unresolved_rosters: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut roster_present: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut event_athletes: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

    let source_prefix_len = source_prefix.len();
    for guard in snap.prefix(&store.rankings, source_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rid) = decode_be_u64(&key, source_prefix_len) {
            let _ = source_results.insert(rid);
        }
    }

    let row_prefix = row_position_prefix(collection, event_short);
    let row_prefix_len = row_prefix.len();
    for guard in snap.prefix(&store.rankings, row_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rn) = decode_be_u64(&key, row_prefix_len + 8 + 1) {
            row_positions += 1;
            if rn > max_row_position {
                max_row_position = rn;
            }
        }
    }

    let eligible_prefix = eligible_individual_prefix(collection, event_short);
    let eligible_prefix_len = eligible_prefix.len();
    for guard in snap.prefix(&store.rankings, eligible_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rid) = decode_be_u64(&key, eligible_prefix_len) {
            let _ = grade11_individual.insert(rid);
        }
    }

    let relay_prefix = eligible_relay_prefix(collection, event_short);
    let relay_prefix_len = relay_prefix.len();
    for guard in snap.prefix(&store.rankings, relay_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rid) = decode_be_u64(&key, relay_prefix_len) {
            if let Some(aid) = decode_be_u64(&key, relay_prefix_len + 8 + 1) {
                let _ = grade11_relay.insert((rid, aid));
            }
        }
    }

    let roster_present_prefix = roster_present_prefix(collection, event_short);
    let roster_present_prefix_len = roster_present_prefix.len();
    for guard in snap.prefix(&store.rankings, roster_present_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rid) = decode_be_u64(&key, roster_present_prefix_len) {
            let _ = roster_present.insert(rid);
        }
    }

    let roster_missing_prefix = roster_missing_prefix(collection, event_short);
    let roster_missing_prefix_len = roster_missing_prefix.len();
    for guard in snap.prefix(&store.rankings, roster_missing_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(rid) = decode_be_u64(&key, roster_missing_prefix_len) {
            let _ = unresolved_rosters.insert(rid);
        }
    }

    // Count unique athletes in this event.
    let mut athlete_prefix = Vec::new();
    athlete_prefix.extend_from_slice(COLLECTION_PREFIX);
    athlete_prefix.extend_from_slice(b"re\0");
    athlete_prefix.extend_from_slice(collection.as_str().as_bytes());
    athlete_prefix.push(b'\0');
    athlete_prefix.extend_from_slice(event_short.as_bytes());
    athlete_prefix.push(b'\0');
    let athlete_prefix_len = athlete_prefix.len();
    for guard in snap.prefix(&store.rankings, athlete_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(aid) = decode_be_u64(&key, athlete_prefix_len) {
            if aid > 0 {
                let _ = event_athletes.insert(aid);
            }
        }
    }

    let unresolved_count = unresolved_rosters
        .into_iter()
        .filter(|rid| !roster_present.contains(rid))
        .count() as u64;

    Ok(RankingEventStats {
        pages,
        source_results: source_results.len() as u64,
        row_positions,
        max_row_position,
        grade11_individual_results: grade11_individual.len() as u64,
        grade11_relay_member_results: grade11_relay.len() as u64,
        unique_athletes: event_athletes.len() as u64,
        unresolved_roster_results: unresolved_count,
    })
}

/// Count distinct real athletes across a collection.
pub(in crate::store) fn ranking_collection_stats(
    store: &StoreInner,
    collection: &EvidenceDigest,
) -> Result<RankingCollectionStats, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    let snap = store.database.snapshot();
    // Scan event-athlete keys: rk\0re\0<collection>\0<event>\0<athlete(8B)>.
    let prefix = super::keys::event_athlete_prefix(collection);
    let prefix_len = prefix.len();

    let mut unique: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        if let Some(aid) = decode_be_u64(&key, prefix_len) {
            if aid > 0 {
                let _ = unique.insert(aid);
            }
        }
    }

    Ok(RankingCollectionStats {
        unique_athletes: unique.len() as u64,
    })
}
