use super::keys::{
    eligible_individual_prefix, eligible_relay_prefix, event_athlete_prefix, roster_missing_prefix,
    roster_present_prefix, row_position_prefix, source_result_prefix, validate_event_short,
};
use super::types::{RankingCollectionStats, RankingEventStats};
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::Readable;

/// Decode a fixed-width big-endian u64 at the given offset from the key.
fn decode_be_u64(key: &[u8], offset: usize) -> Result<u64, StoreError> {
    let end = offset.checked_add(8).ok_or(StoreError::CorruptData)?;
    let slice = key.get(offset..end).ok_or(StoreError::CorruptData)?;
    let bytes: [u8; 8] = slice.try_into().map_err(|_| StoreError::CorruptData)?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_suffix(key: &[u8], prefix_len: usize) -> Result<u64, StoreError> {
    let expected_len = prefix_len.checked_add(8).ok_or(StoreError::CorruptData)?;
    if key.len() != expected_len {
        return Err(StoreError::CorruptData);
    }
    let value = decode_be_u64(key, prefix_len)?;
    (value != 0).then_some(value).ok_or(StoreError::CorruptData)
}

fn decode_pair(key: &[u8], prefix_len: usize) -> Result<(u64, u64), StoreError> {
    let separator = prefix_len.checked_add(8).ok_or(StoreError::CorruptData)?;
    let end = separator.checked_add(9).ok_or(StoreError::CorruptData)?;
    if key.len() != end || key.get(separator) != Some(&0) {
        return Err(StoreError::CorruptData);
    }
    let first = decode_be_u64(key, prefix_len)?;
    let second = decode_be_u64(key, separator + 1)?;
    if first == 0 || second == 0 {
        return Err(StoreError::CorruptData);
    }
    Ok((first, second))
}

fn next_count(count: u64) -> Result<u64, StoreError> {
    count.checked_add(1).ok_or(StoreError::CorruptData)
}
fn decode_event_athlete(key: &[u8], prefix_len: usize) -> Result<u64, StoreError> {
    let suffix = key.get(prefix_len..).ok_or(StoreError::CorruptData)?;
    let separator = suffix.len().checked_sub(9).ok_or(StoreError::CorruptData)?;
    if suffix.get(separator) != Some(&0) {
        return Err(StoreError::CorruptData);
    }
    let event = std::str::from_utf8(suffix.get(..separator).ok_or(StoreError::CorruptData)?)
        .map_err(|_| StoreError::CorruptData)?;
    validate_event_short(event).map_err(|_| StoreError::CorruptData)?;
    let athlete = decode_be_u64(suffix, separator + 1)?;
    (athlete != 0)
        .then_some(athlete)
        .ok_or(StoreError::CorruptData)
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
    let page_prefix_len = page_prefix.len();
    let mut pages = 0_u64;
    for guard in snap.prefix(&store.rankings, page_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let suffix = key.get(page_prefix_len..).ok_or(StoreError::CorruptData)?;
        let bytes: [u8; 4] = suffix.try_into().map_err(|_| StoreError::CorruptData)?;
        if u32::from_be_bytes(bytes) == 0 {
            return Err(StoreError::CorruptData);
        }
        pages = next_count(pages)?;
    }

    let source_prefix = source_result_prefix(collection, event_short);
    let source_prefix_len = source_prefix.len();
    let mut source_results = 0_u64;
    for guard in snap.prefix(&store.rankings, source_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let _result = decode_suffix(&key, source_prefix_len)?;
        source_results = next_count(source_results)?;
    }

    let row_prefix = row_position_prefix(collection, event_short);
    let row_prefix_len = row_prefix.len();
    let mut positions = std::collections::BTreeSet::new();
    let mut max_row_position = 0_u64;
    for guard in snap.prefix(&store.rankings, row_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (_result, position) = decode_pair(&key, row_prefix_len)?;
        let _ = positions.insert(position);
        max_row_position = max_row_position.max(position);
    }
    let row_positions = u64::try_from(positions.len()).map_err(|_| StoreError::CorruptData)?;

    let individual_prefix = eligible_individual_prefix(collection, event_short);
    let individual_prefix_len = individual_prefix.len();
    let mut grade11_individual_results = 0_u64;
    let mut last_individual_result = None;
    for guard in snap.prefix(&store.rankings, individual_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (result, _athlete) = decode_pair(&key, individual_prefix_len)?;
        if last_individual_result != Some(result) {
            grade11_individual_results = next_count(grade11_individual_results)?;
            last_individual_result = Some(result);
        }
    }

    let relay_prefix = eligible_relay_prefix(collection, event_short);
    let relay_prefix_len = relay_prefix.len();
    let mut grade11_relay_member_results = 0_u64;
    for guard in snap.prefix(&store.rankings, relay_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let _pair = decode_pair(&key, relay_prefix_len)?;
        grade11_relay_member_results = next_count(grade11_relay_member_results)?;
    }

    let present_prefix = roster_present_prefix(collection, event_short);
    let present_prefix_len = present_prefix.len();
    for guard in snap.prefix(&store.rankings, &present_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let _result = decode_suffix(&key, present_prefix_len)?;
    }
    let missing_prefix = roster_missing_prefix(collection, event_short);
    let missing_prefix_len = missing_prefix.len();
    let mut unresolved_roster_results = 0_u64;
    let mut present_key = present_prefix;
    for guard in snap.prefix(&store.rankings, missing_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let result = decode_suffix(&key, missing_prefix_len)?;
        present_key.truncate(present_prefix_len);
        present_key.extend_from_slice(&result.to_be_bytes());
        let found = snap
            .get(&store.rankings, &present_key)
            .map_err(|_| StoreError::CorruptData)?;
        if found.is_none() {
            unresolved_roster_results = next_count(unresolved_roster_results)?;
        }
    }

    let mut athlete_prefix = event_athlete_prefix(collection);
    athlete_prefix.extend_from_slice(event_short.as_bytes());
    athlete_prefix.push(0);
    let athlete_prefix_len = athlete_prefix.len();
    let mut unique_athletes = 0_u64;
    for guard in snap.prefix(&store.rankings, athlete_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let _athlete = decode_suffix(&key, athlete_prefix_len)?;
        unique_athletes = next_count(unique_athletes)?;
    }

    Ok(RankingEventStats {
        pages,
        source_results,
        row_positions,
        max_row_position,
        grade11_individual_results,
        grade11_relay_member_results,
        unique_athletes,
        unresolved_roster_results,
    })
}

/// Count distinct real athletes across a collection.
pub(in crate::store) fn ranking_collection_stats(
    store: &StoreInner,
    collection: &EvidenceDigest,
) -> Result<RankingCollectionStats, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;
    let snap = store.database.snapshot();
    let prefix = event_athlete_prefix(collection);
    let prefix_len = prefix.len();
    let mut unique = std::collections::BTreeSet::new();

    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let athlete = decode_event_athlete(&key, prefix_len)?;
        let _ = unique.insert(athlete);
    }

    let unique_athletes = u64::try_from(unique.len()).map_err(|_| StoreError::CorruptData)?;
    Ok(RankingCollectionStats { unique_athletes })
}
