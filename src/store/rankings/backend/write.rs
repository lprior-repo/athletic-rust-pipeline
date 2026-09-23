//! Page publication: admit or clear a page marker and validate page bounds.
use super::super::keys::{
    page_marker_key, page_marker_value, parse_page_marker_value, revoked_capture_key,
    validate_event_short,
};
use super::super::types::{RankingCandidateKind, RankingPageIndex};
use super::batch::{
    insert_candidate_presence, insert_event_athletes, insert_roster_presence, insert_row_presence,
};
use super::seal::load_seal;
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_CANDIDATES: usize = 1_024;
const MAX_ROWS: usize = 1_024;
const MAX_ROSTERS: usize = 1_024;

/// Clear a page marker that no completed publication backs.
///
/// A page step that fails after its index write leaves a marker for evidence the
/// run never sealed. Restate re-executes the whole step after a pause, so the
/// retry arrives with a fresh and equally valid capture of the same page, which
/// the stale marker would reject forever. The run advances past every page it
/// publishes, so a finished publication is never re-published: an existing
/// marker for the page currently being published can only be abandoned state.
///
/// The staged keys of the abandoned capture stay behind, and that is not a
/// retry hazard: every staged key carries the checkpoint of the capture that
/// wrote it, and the republished marker accepts the new capture's checkpoint, so
/// the abandoned keys are unreferenced garbage no reader counts or returns. The
/// dropped marker's own value is kept as a revoked capture entry, which refuses
/// only the one reuse that could mix the two captures: the same checkpoint
/// carrying other content. Removing the abandoned keys instead would be an
/// enumeration of live index data, and the seal check below is what keeps this a
/// recovery maneuver rather than an edit of a sealed collection.
pub(in crate::store) fn drop_rankings_page(
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    page: u32,
) -> Result<bool, StoreError> {
    validate_event_short(event_short)?;
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;
    if load_seal(&store.rankings, collection)?.is_some() {
        return Err(StoreError::RankingConflict);
    }
    let key = page_marker_key(collection, event_short, page)?;
    let Some(value) = store
        .rankings
        .get(&key)
        .map_err(|_| StoreError::CorruptData)?
    else {
        return Ok(false);
    };
    // A marker whose value does not decode is corruption: the capture it
    // certifies cannot be revoked, so this must not clear it silently.
    let (checkpoint, index_hash) = parse_page_marker_value(value.as_ref())?;
    let revoked_key = revoked_capture_key(collection, &checkpoint)?;

    // The database is opened with `manual_journal_persist(true)`, so a
    // keyspace-level removal would sit unpersisted in the journal until the
    // process closes the database cleanly. The removal is committed the way
    // publication is, or a recovery maneuver that crashed would find the stale
    // marker it thought it had cleared still certifying the page.
    let mut batch = store.database.batch();
    batch.remove(&store.rankings, key);
    batch.insert(&store.rankings, revoked_key, index_hash.to_vec());
    batch
        .durability(Some(fjall::PersistMode::SyncAll))
        .commit()
        .map_err(|_| StoreError::CorruptData)?;
    Ok(true)
}

pub(in crate::store) fn put_rankings_page(
    store: &StoreInner,
    index: &RankingPageIndex,
) -> Result<(), StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    validate_page_index(index)?;

    // Verify checkpoint document exists and is valid BEFORE publishing.
    let checkpoint_bytes = store.get_bytes(&index.checkpoint)?;
    if checkpoint_bytes.is_empty() {
        return Err(StoreError::MissingArtifact);
    }

    let serialized = serde_json::to_vec(index).map_err(|_| StoreError::Serialization)?;
    let index_hash = Sha256::digest(&serialized);

    let marker_key = page_marker_key(&index.collection, &index.event_short, index.page)?;
    let marker_value = page_marker_value(&index.checkpoint, index_hash.as_slice())?;

    if admit_page(store, index, &marker_key, index_hash.as_slice())? {
        return Ok(());
    }

    let mut batch = store.database.batch();
    // Saturating adds below: saturation always trips the MAX_RANKINGS_BATCH_BYTES check,
    // while a wrapped count could slip past it.
    let mut total_batch_bytes: usize = serialized.len();

    batch.insert(&store.rankings, marker_key, marker_value);
    total_batch_bytes = total_batch_bytes.saturating_add(serialized.len());

    insert_row_presence(store, index, &mut batch, &mut total_batch_bytes)?;
    let event_athletes =
        insert_candidate_presence(store, index, &mut batch, &mut total_batch_bytes)?;
    insert_roster_presence(store, index, &mut batch, &mut total_batch_bytes)?;
    insert_event_athletes(
        store,
        index,
        &event_athletes,
        &mut batch,
        &mut total_batch_bytes,
    )?;

    // Bound total serialized batch bytes.
    const MAX_RANKINGS_BATCH_BYTES: usize = 32 * 1024 * 1024;
    if total_batch_bytes > MAX_RANKINGS_BATCH_BYTES {
        return Err(StoreError::BatchTooLarge);
    }

    batch
        .durability(Some(fjall::PersistMode::SyncAll))
        .commit()
        .map_err(|_| StoreError::CorruptData)?;
    Ok(())
}

/// Reports whether this page is already published, and refuses the two ways a
/// publication could put two captures under one accepted checkpoint.
fn admit_page(
    store: &StoreInner,
    index: &RankingPageIndex,
    marker_key: &[u8],
    index_hash: &[u8],
) -> Result<bool, StoreError> {
    // Check for page conflict or replay no-op.
    if let Some(existing) = store
        .rankings
        .get(marker_key)
        .map_err(|_| StoreError::CorruptData)?
    {
        let (checkpoint, stored_hash) = parse_page_marker_value(existing.as_ref())?;
        return if checkpoint == index.checkpoint && stored_hash == index_hash {
            Ok(true)
        } else {
            Err(StoreError::RankingConflict)
        };
    }

    // A capture whose marker was dropped left its index keys stored, so reusing
    // its checkpoint for other content would count both captures. Reusing it for
    // the same content writes the same keys, which cannot mix anything.
    let revoked_key = revoked_capture_key(&index.collection, &index.checkpoint)?;
    if let Some(revoked_hash) = store
        .rankings
        .get(&revoked_key)
        .map_err(|_| StoreError::CorruptData)?
    {
        if revoked_hash.as_ref() != index_hash {
            return Err(StoreError::RankingConflict);
        }
    }

    // Check seal after page check so we allow replay after normal seal.
    if load_seal(&store.rankings, &index.collection)?.is_some() {
        return Err(StoreError::RankingConflict);
    }
    Ok(false)
}

/// Reject every page the stats decoders would later call corrupt, plus the
/// relationships inside a page the index depends on.
///
/// The write contract and the read contract describe the same page, so a value
/// that `backend_stats` refuses to decode must never reach the batch: a zero
/// result id or row number, a candidate or roster result that names no source
/// row of this page, and a candidate that cannot be told apart from another
/// candidate of the same page (same kind and record index).
fn validate_page_index(index: &RankingPageIndex) -> Result<(), StoreError> {
    if index.page == 0 || index.page > 10_000 {
        return Err(StoreError::InvalidRankingInput);
    }
    if index.rows.len() > MAX_ROWS
        || index.candidates.len() > MAX_CANDIDATES
        || index.rosters.len() > MAX_ROSTERS
    {
        return Err(StoreError::RankingConflict);
    }
    validate_event_short(&index.event_short)?;

    let mut result_ids: BTreeSet<u64> = BTreeSet::new();
    for row in &index.rows {
        if row.result_id == 0 || row.row_number == 0 {
            return Err(StoreError::InvalidRankingInput);
        }
        let _ = result_ids.insert(row.result_id);
    }

    let mut candidate_records: BTreeSet<(RankingCandidateKind, u32)> = BTreeSet::new();
    for entry in &index.candidates {
        if entry.result_id == 0 || !result_ids.contains(&entry.result_id) {
            return Err(StoreError::InvalidRankingInput);
        }
        if !candidate_records.insert((entry.kind, entry.record_index)) {
            return Err(StoreError::InvalidRankingInput);
        }
    }

    let mut roster_presence: BTreeMap<u64, bool> = BTreeMap::new();
    for roster in &index.rosters {
        if roster.result_id == 0 || !result_ids.contains(&roster.result_id) {
            return Err(StoreError::InvalidRankingInput);
        }
        if let Some(previous) = roster_presence.insert(roster.result_id, roster.present) {
            if previous != roster.present {
                return Err(StoreError::InvalidRankingInput);
            }
        }
    }
    Ok(())
}
