//! Batch staging: stage the presence keys and reference pointers one page writes.
use super::super::keys::{
    athlete_ref_key, name_ref_key, presence_eligible_individual, presence_eligible_relay_member,
    presence_roster_missing, presence_roster_present, presence_row_position,
    presence_source_result,
};
use super::super::presence_keys::presence_event_athlete;
use super::super::types::{
    RankingCandidateEntry, RankingCandidateKind, RankingPageIndex, RankingRecordRef,
};
use crate::domain::identity::AthleteId;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::OwnedWriteBatch;

/// Inserts the source-result and row-position presence keys for one page.
pub(super) fn insert_row_presence(
    store: &StoreInner,
    index: &RankingPageIndex,
    batch: &mut OwnedWriteBatch,
    total_batch_bytes: &mut usize,
) -> Result<(), StoreError> {
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
        *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
        batch.insert(&store.rankings, key, []);
    }

    for row in &index.rows {
        let key = presence_row_position(
            &index.collection,
            &index.event_short,
            row.result_id,
            row.row_number,
        )?;
        *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
        batch.insert(&store.rankings, key, []);
    }
    Ok(())
}

/// Inserts the candidate reference and eligible presence keys, returning event athletes.
pub(super) fn insert_candidate_presence(
    store: &StoreInner,
    index: &RankingPageIndex,
    batch: &mut OwnedWriteBatch,
    total_batch_bytes: &mut usize,
) -> Result<std::collections::BTreeSet<u64>, StoreError> {
    // Event-level athlete presence keys (were missing, causing zero unique counters).
    let mut event_athletes: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut grade11_individual: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut grade11_relay: std::collections::BTreeSet<(u64, u64)> =
        std::collections::BTreeSet::new();

    for entry in &index.candidates {
        insert_candidate_refs(store, index, entry, batch, total_batch_bytes)?;

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
                *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
                batch.insert(&store.rankings, key, []);
            }
            RankingCandidateKind::RelayMember => {
                let _ = grade11_relay.insert((entry.result_id, entry.athlete_id.get()));
                let key = presence_eligible_relay_member(
                    &index.collection,
                    &index.event_short,
                    entry.result_id,
                    entry.athlete_id,
                )?;
                *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
                batch.insert(&store.rankings, key, []);
            }
        }
    }
    Ok(event_athletes)
}

/// Inserts the athlete and name reference keys for one candidate.
pub(super) fn insert_candidate_refs(
    store: &StoreInner,
    index: &RankingPageIndex,
    entry: &RankingCandidateEntry,
    batch: &mut OwnedWriteBatch,
    total_batch_bytes: &mut usize,
) -> Result<(), StoreError> {
    let ref_entry = RankingRecordRef {
        athlete_id: entry.athlete_id,
        checkpoint: index.checkpoint.clone(),
        kind: entry.kind,
        record_index: entry.record_index,
    };

    // Serialize once and share value for both inserts.
    let ref_serialized = serde_json::to_vec(&ref_entry).map_err(|_| StoreError::Serialization)?;
    let ref_bytes = ref_serialized.as_slice();

    let ref_key = athlete_ref_key(&index.collection, &ref_entry)?;
    *total_batch_bytes =
        total_batch_bytes.saturating_add(ref_key.len().saturating_add(ref_bytes.len()));
    batch.insert(&store.rankings, ref_key, ref_bytes);

    let name_key = name_ref_key(
        &index.collection,
        entry.name.as_str(),
        entry.athlete_id,
        &index.checkpoint,
        entry.kind,
        entry.record_index,
    )?;
    *total_batch_bytes =
        total_batch_bytes.saturating_add(name_key.len().saturating_add(ref_bytes.len()));
    batch.insert(&store.rankings, name_key, ref_bytes);
    Ok(())
}

/// Inserts the roster present and missing presence keys for one page.
pub(super) fn insert_roster_presence(
    store: &StoreInner,
    index: &RankingPageIndex,
    batch: &mut OwnedWriteBatch,
    total_batch_bytes: &mut usize,
) -> Result<(), StoreError> {
    let mut roster_missing: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut roster_present: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

    for roster in &index.rosters {
        if roster.present {
            let _ = roster_present.insert(roster.result_id);
            let key =
                presence_roster_present(&index.collection, &index.event_short, roster.result_id)?;
            *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
            batch.insert(&store.rankings, key, []);
        } else {
            let _ = roster_missing.insert(roster.result_id);
            let key =
                presence_roster_missing(&index.collection, &index.event_short, roster.result_id)?;
            *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
            batch.insert(&store.rankings, key, []);
        }
    }
    Ok(())
}

/// Writes event-level athlete presence keys.
pub(super) fn insert_event_athletes(
    store: &StoreInner,
    index: &RankingPageIndex,
    event_athletes: &std::collections::BTreeSet<u64>,
    batch: &mut OwnedWriteBatch,
    total_batch_bytes: &mut usize,
) -> Result<(), StoreError> {
    for athlete_id in event_athletes {
        let key = presence_event_athlete(
            &index.collection,
            &index.event_short,
            AthleteId::try_from(*athlete_id).map_err(|_| StoreError::InvalidRankingInput)?,
        )?;
        *total_batch_bytes = total_batch_bytes.saturating_add(key.len());
        batch.insert(&store.rankings, key, []);
    }
    Ok(())
}
