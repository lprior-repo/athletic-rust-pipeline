use super::keys::*;
use super::types::*;
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::{Keyspace, Readable};
use sha2::{Digest, Sha256};

const MAX_CANDIDATES: usize = 1_024;
const MAX_ROWS: usize = 1_024;
const MAX_ROSTERS: usize = 1_024;

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

    // Check for page conflict or replay no-op.
    if store
        .rankings
        .get(&marker_key)
        .map_err(|_| StoreError::CorruptData)?
        .is_some()
    {
        return Err(StoreError::RankingConflict);
    }

    // Check seal after page check so we allow replay after normal seal.
    if load_seal(&store.rankings, &index.collection)?.is_some() {
        return Err(StoreError::RankingConflict);
    }

    let mut batch = store.database.batch();
    let mut total_batch_bytes: usize = serialized.len();

    batch.insert(&store.rankings, marker_key, index_hash.as_slice());
    total_batch_bytes += serialized.len();

    let mut source_results: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut max_row_position: u64 = 0;

    for row in &index.rows {
        let _ = source_results.insert(row.result_id);
        if row.row_number > max_row_position {
            max_row_position = row.row_number;
        }
    }

    for result_id in &source_results {
        let key = presence_source_result(&index.collection, &index.event_short, *result_id)?;
        total_batch_bytes += key.len();
        batch.insert(&store.rankings, key, &[]);
    }

    for row in &index.rows {
        let key = presence_row_position(
            &index.collection,
            &index.event_short,
            row.result_id,
            row.row_number,
        )?;
        total_batch_bytes += key.len();
        batch.insert(&store.rankings, key, &[]);
    }

    // Event-level athlete presence keys (were missing, causing zero unique counters).
    let mut event_athletes: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut grade11_individual: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut grade11_relay: std::collections::BTreeSet<(u64, u64)> =
        std::collections::BTreeSet::new();
    let mut roster_missing: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut roster_present: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

    for entry in &index.candidates {
        let ref_entry = RankingRecordRef {
            athlete_id: entry.athlete_id,
            checkpoint: index.checkpoint.clone(),
            kind: entry.kind,
            record_index: entry.record_index,
        };

        // Serialize once and share value for both inserts.
        let ref_serialized =
            serde_json::to_vec(&ref_entry).map_err(|_| StoreError::Serialization)?;
        let ref_bytes = ref_serialized.as_slice();

        let ref_key = athlete_ref_key(&index.collection, &ref_entry)?;
        total_batch_bytes += ref_key.len() + ref_bytes.len();
        batch.insert(&store.rankings, ref_key, ref_bytes);

        let name_key = name_ref_key(
            &index.collection,
            entry.name.as_str(),
            entry.athlete_id,
            &index.checkpoint,
            entry.kind,
            entry.record_index,
        )?;
        total_batch_bytes += name_key.len() + ref_bytes.len();
        batch.insert(&store.rankings, name_key, ref_bytes);

        // Track event-level athlete for stats.
        let _ = event_athletes.insert(entry.athlete_id.get());

        match entry.kind {
            RankingCandidateKind::Individual => {
                let _ = grade11_individual.insert(entry.result_id);
                let key = presence_eligible_individual(
                    &index.collection,
                    &index.event_short,
                    entry.result_id,
                    entry.athlete_id,
                )?;
                total_batch_bytes += key.len();
                batch.insert(&store.rankings, key, &[]);
            }
            RankingCandidateKind::RelayMember => {
                let _ = grade11_relay.insert((entry.result_id, entry.athlete_id.get()));
                let key = presence_eligible_relay_member(
                    &index.collection,
                    &index.event_short,
                    entry.result_id,
                    entry.athlete_id,
                )?;
                total_batch_bytes += key.len();
                batch.insert(&store.rankings, key, &[]);
            }
        }
    }

    for roster in &index.rosters {
        if roster.present {
            let _ = roster_present.insert(roster.result_id);
            let key =
                presence_roster_present(&index.collection, &index.event_short, roster.result_id)?;
            total_batch_bytes += key.len();
            batch.insert(&store.rankings, key, &[]);
        } else {
            let _ = roster_missing.insert(roster.result_id);
            let key =
                presence_roster_missing(&index.collection, &index.event_short, roster.result_id)?;
            total_batch_bytes += key.len();
            batch.insert(&store.rankings, key, &[]);
        }
    }

    // Write event-level athlete presence keys.
    for athlete_id in &event_athletes {
        let key = super::presence_keys::presence_event_athlete(
            &index.collection,
            &index.event_short,
            AthleteId::try_from(*athlete_id).map_err(|_| StoreError::InvalidRankingInput)?,
        )?;
        total_batch_bytes += key.len();
        batch.insert(&store.rankings, key, &[]);
    }

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

pub(in crate::store) fn ranking_name_refs(
    store: &StoreInner,
    collection: &EvidenceDigest,
    name: &CanonicalName,
    limit: usize,
) -> Result<RankingLookup, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    if limit < 1 || limit > 4096 {
        return Err(StoreError::InvalidPageLimit);
    }

    let snap = store.database.snapshot();
    let prefix = name_ref_prefix(collection, name.as_str());
    let mut records = Vec::new();
    let need = limit + 1;

    for guard in snap.prefix(&store.rankings, prefix) {
        let (_, value) = guard.into_inner().map_err(|_| StoreError::CorruptData)?;
        if let Ok(ref_entry) = serde_json::from_slice::<RankingRecordRef>(value.as_ref()) {
            records.push(ref_entry);
        }
        if records.len() >= need {
            break;
        }
    }

    let truncated = records.len() > limit;
    if truncated {
        records.truncate(limit);
    }

    Ok(RankingLookup { records, truncated })
}

pub(in crate::store) fn ranking_athlete_refs(
    store: &StoreInner,
    collection: &EvidenceDigest,
    athlete: AthleteId,
    limit: usize,
) -> Result<RankingLookup, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    if limit < 1 || limit > 4096 {
        return Err(StoreError::InvalidPageLimit);
    }

    let snap = store.database.snapshot();
    let prefix = athlete_ref_prefix(collection, athlete);
    let mut records = Vec::new();
    let need = limit + 1;

    for guard in snap.prefix(&store.rankings, prefix) {
        let (_, value) = guard.into_inner().map_err(|_| StoreError::CorruptData)?;
        if let Ok(ref_entry) = serde_json::from_slice::<RankingRecordRef>(value.as_ref()) {
            records.push(ref_entry);
        }
        if records.len() >= need {
            break;
        }
    }

    let truncated = records.len() > limit;
    if truncated {
        records.truncate(limit);
    }

    Ok(RankingLookup { records, truncated })
}

pub(in crate::store) fn seal_rankings(
    store: &StoreInner,
    collection: &EvidenceDigest,
    snapshot: &EvidenceDigest,
) -> Result<(), StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    if let Some(existing) = load_seal(&store.rankings, collection)? {
        if existing == *snapshot {
            return Ok(());
        }
        return Err(StoreError::RankingConflict);
    }

    // Verify snapshot document exists and is valid BEFORE sealing.
    let snapshot_bytes = store.get_bytes(snapshot)?;
    if snapshot_bytes.is_empty() {
        return Err(StoreError::MissingArtifact);
    }

    let seal_key = seal_key(collection);
    let mut batch = store.database.batch();
    batch.insert(&store.rankings, seal_key, snapshot.as_str().as_bytes());
    batch
        .durability(Some(fjall::PersistMode::SyncAll))
        .commit()
        .map_err(|_| StoreError::CorruptData)?;

    Ok(())
}

pub(in crate::store) fn ranking_snapshot(
    store: &StoreInner,
    collection: &EvidenceDigest,
) -> Result<Option<EvidenceDigest>, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    let key = seal_key(collection);
    match store
        .rankings
        .get(&key)
        .map_err(|_| StoreError::CorruptData)?
    {
        Some(value) => {
            let text =
                String::from_utf8(value.as_ref().to_vec()).map_err(|_| StoreError::CorruptData)?;
            EvidenceDigest::parse(&text)
                .map(Some)
                .map_err(|_| StoreError::CorruptData)
        }
        None => Ok(None),
    }
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

fn load_seal(
    ks: &Keyspace,
    collection: &EvidenceDigest,
) -> Result<Option<EvidenceDigest>, StoreError> {
    let key = seal_key(collection);
    match ks.get(&key).map_err(|_| StoreError::CorruptData)? {
        Some(value) => {
            let text =
                String::from_utf8(value.as_ref().to_vec()).map_err(|_| StoreError::CorruptData)?;
            EvidenceDigest::parse(&text)
                .map(Some)
                .map_err(|_| StoreError::CorruptData)
        }
        None => Ok(None),
    }
}
