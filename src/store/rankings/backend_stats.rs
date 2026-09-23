//! Event and collection statistics: streaming key-prefix scans of the accepted
//! page captures only.
use super::common::DIGEST_BYTES;
use super::keys::{
    collection_page_prefix, eligible_individual_prefix, eligible_relay_prefix,
    event_athlete_prefix, roster_missing_prefix, roster_present_prefix, row_position_prefix,
    source_result_prefix, validate_event_short,
};
use super::page_identity::{accepted_pages, checkpoint_at, AcceptedPages};
use super::types::{RankingCollectionStats, RankingEventStats};
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::{Readable, Snapshot};

/// Decode a fixed-width big-endian u64 at the given offset from the key.
fn decode_be_u64(key: &[u8], offset: usize) -> Result<u64, StoreError> {
    let end = offset.checked_add(8).ok_or(StoreError::CorruptData)?;
    let slice = key.get(offset..end).ok_or(StoreError::CorruptData)?;
    let bytes: [u8; 8] = slice.try_into().map_err(|_| StoreError::CorruptData)?;
    Ok(u64::from_be_bytes(bytes))
}

/// Offset of the segment that follows `<checkpoint>\0` at `offset`.
fn after_checkpoint(offset: usize) -> Result<usize, StoreError> {
    offset
        .checked_add(DIGEST_BYTES)
        .and_then(|end| end.checked_add(1))
        .ok_or(StoreError::CorruptData)
}

/// Decode `<checkpoint><0><u64>` at `offset`, rejecting zero and stray bytes.
fn decode_checkpoint_value(key: &[u8], offset: usize) -> Result<(&[u8], u64), StoreError> {
    let checkpoint = checkpoint_at(key, offset)?;
    let value_offset = after_checkpoint(offset)?;
    let expected_len = value_offset.checked_add(8).ok_or(StoreError::CorruptData)?;
    if key.len() != expected_len {
        return Err(StoreError::CorruptData);
    }
    let value = decode_be_u64(key, value_offset)?;
    (value != 0)
        .then_some((checkpoint, value))
        .ok_or(StoreError::CorruptData)
}

/// Decode `<checkpoint><0><u64><0><u64>` at `offset`, rejecting zeros.
fn decode_checkpoint_pair(key: &[u8], offset: usize) -> Result<(&[u8], u64, u64), StoreError> {
    let checkpoint = checkpoint_at(key, offset)?;
    let first_offset = after_checkpoint(offset)?;
    let separator = first_offset.checked_add(8).ok_or(StoreError::CorruptData)?;
    let second_offset = separator.checked_add(1).ok_or(StoreError::CorruptData)?;
    let expected_len = second_offset
        .checked_add(8)
        .ok_or(StoreError::CorruptData)?;
    if key.len() != expected_len || key.get(separator) != Some(&0) {
        return Err(StoreError::CorruptData);
    }
    let first = decode_be_u64(key, first_offset)?;
    let second = decode_be_u64(key, second_offset)?;
    (first != 0 && second != 0)
        .then_some((checkpoint, first, second))
        .ok_or(StoreError::CorruptData)
}

fn next_count(count: u64) -> Result<u64, StoreError> {
    count.checked_add(1).ok_or(StoreError::CorruptData)
}

/// Decode the `<checkpoint><0><athlete(8B)>` tail of an event-athlete key.
fn decode_event_athlete(key: &[u8], offset: usize) -> Result<(&[u8], u64), StoreError> {
    let suffix = key.get(offset..).ok_or(StoreError::CorruptData)?;
    let separator = suffix.iter().position(|byte| *byte == 0);
    let event_end = separator.ok_or(StoreError::CorruptData)?;
    let event = std::str::from_utf8(suffix.get(..event_end).ok_or(StoreError::CorruptData)?)
        .map_err(|_| StoreError::CorruptData)?;
    validate_event_short(event).map_err(|_| StoreError::CorruptData)?;
    let checkpoint_offset = event_end.checked_add(1).ok_or(StoreError::CorruptData)?;
    let (checkpoint, athlete) = decode_checkpoint_value(suffix, checkpoint_offset)?;
    Ok((checkpoint, athlete))
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

    // The accepted captures are resolved from every marker of the collection and
    // then filtered by event: a marker of another event accepts no key of this one.
    let accepted = accepted_pages(&snap, store, &collection_page_prefix(collection))?;
    let source_results = source_result_count(&snap, store, collection, event_short, &accepted)?;
    let (row_positions, max_row_position) =
        row_position_stats(&snap, store, collection, event_short, &accepted)?;
    let (grade11_individual_results, grade11_relay_member_results) =
        eligible_counts(&snap, store, collection, event_short, &accepted)?;
    let unresolved_roster_results =
        unresolved_roster_count(&snap, store, collection, event_short, &accepted)?;
    let unique_athletes = unique_athlete_count(&snap, store, collection, event_short, &accepted)?;

    Ok(RankingEventStats {
        pages: accepted.pages_for(event_short),
        source_results,
        row_positions,
        max_row_position,
        grade11_individual_results,
        grade11_relay_member_results,
        unique_athletes,
        unresolved_roster_results,
    })
}

/// Counts distinct source results of the accepted captures for one event.
fn source_result_count(
    snap: &Snapshot,
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    accepted: &AcceptedPages,
) -> Result<u64, StoreError> {
    let prefix = source_result_prefix(collection, event_short);
    let offset = prefix.len();
    let mut source_results = 0_u64;
    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, _result) = decode_checkpoint_value(&key, offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        source_results = next_count(source_results)?;
    }
    Ok(source_results)
}

/// Counts distinct row positions and the highest one for one event.
fn row_position_stats(
    snap: &Snapshot,
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    accepted: &AcceptedPages,
) -> Result<(u64, u64), StoreError> {
    let prefix = row_position_prefix(collection, event_short);
    let offset = prefix.len();
    let mut positions = std::collections::BTreeSet::new();
    let mut max_row_position = 0_u64;
    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, _result, position) = decode_checkpoint_pair(&key, offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        let _ = positions.insert(position);
        max_row_position = max_row_position.max(position);
    }
    let row_positions = u64::try_from(positions.len()).map_err(|_| StoreError::CorruptData)?;
    Ok((row_positions, max_row_position))
}

/// Counts eligible grade-11 individual and relay-member results for one event.
fn eligible_counts(
    snap: &Snapshot,
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    accepted: &AcceptedPages,
) -> Result<(u64, u64), StoreError> {
    let individual_prefix = eligible_individual_prefix(collection, event_short);
    let individual_offset = individual_prefix.len();
    let mut grade11_individual_results = 0_u64;
    let mut last_individual_result = None;
    for guard in snap.prefix(&store.rankings, individual_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, result, _athlete) = decode_checkpoint_pair(&key, individual_offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        if last_individual_result != Some(result) {
            grade11_individual_results = next_count(grade11_individual_results)?;
            last_individual_result = Some(result);
        }
    }

    let relay_prefix = eligible_relay_prefix(collection, event_short);
    let relay_offset = relay_prefix.len();
    let mut grade11_relay_member_results = 0_u64;
    for guard in snap.prefix(&store.rankings, relay_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, _result, _athlete) = decode_checkpoint_pair(&key, relay_offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        grade11_relay_member_results = next_count(grade11_relay_member_results)?;
    }
    Ok((grade11_individual_results, grade11_relay_member_results))
}

/// Counts roster results with no matching present-roster key for one event.
fn unresolved_roster_count(
    snap: &Snapshot,
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    accepted: &AcceptedPages,
) -> Result<u64, StoreError> {
    let present_prefix = roster_present_prefix(collection, event_short);
    let present_offset = present_prefix.len();
    for guard in snap.prefix(&store.rankings, &present_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let _present = decode_checkpoint_value(&key, present_offset)?;
    }

    let missing_prefix = roster_missing_prefix(collection, event_short);
    let missing_offset = missing_prefix.len();
    let mut present_key = Vec::with_capacity(
        present_offset
            .saturating_add(DIGEST_BYTES)
            .saturating_add(8)
            .saturating_add(1),
    );
    let mut unresolved_roster_results = 0_u64;
    for guard in snap.prefix(&store.rankings, missing_prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, result) = decode_checkpoint_value(&key, missing_offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        // The present key of this capture, built from the missing key's own
        // checkpoint and result, so a present roster of another capture cannot
        // resolve this one.
        present_key.clear();
        present_key.extend_from_slice(&present_prefix);
        present_key.extend_from_slice(checkpoint);
        present_key.push(0);
        present_key.extend_from_slice(&result.to_be_bytes());
        let found = snap
            .get(&store.rankings, &present_key)
            .map_err(|_| StoreError::CorruptData)?;
        if found.is_none() {
            unresolved_roster_results = next_count(unresolved_roster_results)?;
        }
    }
    Ok(unresolved_roster_results)
}

/// Counts distinct athletes the accepted captures of one event record.
fn unique_athlete_count(
    snap: &Snapshot,
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    accepted: &AcceptedPages,
) -> Result<u64, StoreError> {
    let mut prefix = event_athlete_prefix(collection);
    prefix.extend_from_slice(event_short.as_bytes());
    prefix.push(0);
    let offset = prefix.len();
    let mut unique_athletes = 0_u64;
    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, _athlete) = decode_checkpoint_value(&key, offset)?;
        if !accepted.retains_for(event_short, checkpoint) {
            continue;
        }
        unique_athletes = next_count(unique_athletes)?;
    }
    Ok(unique_athletes)
}

/// Count distinct real athletes across the accepted captures of a collection.
pub(in crate::store) fn ranking_collection_stats(
    store: &StoreInner,
    collection: &EvidenceDigest,
) -> Result<RankingCollectionStats, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;
    let snap = store.database.snapshot();
    let prefix = event_athlete_prefix(collection);
    let prefix_len = prefix.len();
    let accepted = accepted_pages(&snap, store, &collection_page_prefix(collection))?;
    let mut unique = std::collections::BTreeSet::new();

    for guard in snap.prefix(&store.rankings, prefix) {
        let key = guard.key().map_err(|_| StoreError::CorruptData)?;
        let (checkpoint, athlete) = decode_event_athlete(&key, prefix_len)?;
        // The scan crosses every event of the collection, so acceptance is the
        // collection-wide relation.
        if !accepted.retains(checkpoint) {
            continue;
        }
        let _ = unique.insert(athlete);
    }

    let unique_athletes = u64::try_from(unique.len()).map_err(|_| StoreError::CorruptData)?;
    Ok(RankingCollectionStats { unique_athletes })
}
