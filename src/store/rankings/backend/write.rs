//! Page publication: admit or clear a page marker and validate page bounds.
use super::super::keys::{page_marker_key, validate_event_short};
use super::super::types::RankingPageIndex;
use super::batch::{
    insert_candidate_presence, insert_event_athletes, insert_roster_presence, insert_row_presence,
};
use super::seal::load_seal;
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use sha2::{Digest, Sha256};

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
/// Staged row and candidate keys stay; they are idempotent observations of the
/// same page.
pub(in crate::store) fn drop_rankings_page(
    store: &StoreInner,
    collection: &EvidenceDigest,
    event_short: &str,
    page: u32,
) -> Result<bool, StoreError> {
    validate_event_short(event_short)?;
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;
    let key = page_marker_key(collection, event_short, page)?;
    let existed = store
        .rankings
        .get(&key)
        .map_err(|_| StoreError::CorruptData)?
        .is_some();
    if existed {
        store
            .rankings
            .remove(&key)
            .map_err(|_| StoreError::Database)?;
    }
    Ok(existed)
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

    if page_published(store, index, &marker_key, index_hash.as_slice())? {
        return Ok(());
    }

    let mut batch = store.database.batch();
    // Saturating adds below: saturation always trips the MAX_RANKINGS_BATCH_BYTES check,
    // while a wrapped count could slip past it.
    let mut total_batch_bytes: usize = serialized.len();

    batch.insert(&store.rankings, marker_key, index_hash.as_slice());
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

/// Reports whether this exact page is already published, rejecting any other marker.
fn page_published(
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
        return if existing.as_ref() == index_hash {
            Ok(true)
        } else {
            Err(StoreError::RankingConflict)
        };
    }

    // Check seal after page check so we allow replay after normal seal.
    if load_seal(&store.rankings, &index.collection)?.is_some() {
        return Err(StoreError::RankingConflict);
    }
    Ok(false)
}

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
    Ok(())
}
