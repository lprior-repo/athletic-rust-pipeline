//! Reference lookups: read the name and athlete reference indexes of a collection.
use super::super::keys::{athlete_ref_prefix, name_ref_prefix};
use super::super::types::{RankingLookup, RankingRecordRef};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::Readable;

pub(in crate::store) fn ranking_name_refs(
    store: &StoreInner,
    collection: &EvidenceDigest,
    name: &CanonicalName,
    limit: usize,
) -> Result<RankingLookup, StoreError> {
    let _writer = store.writer.lock().map_err(|_| StoreError::Database)?;

    if !(1..=4096).contains(&limit) {
        return Err(StoreError::InvalidPageLimit);
    }

    let snap = store.database.snapshot();
    let prefix = name_ref_prefix(collection, name.as_str());
    let mut records = Vec::new();
    let need = limit.saturating_add(1); // limit is validated to 1..=4096 above

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

    if !(1..=4096).contains(&limit) {
        return Err(StoreError::InvalidPageLimit);
    }

    let snap = store.database.snapshot();
    let prefix = athlete_ref_prefix(collection, athlete);
    let mut records = Vec::new();
    let need = limit.saturating_add(1); // limit is validated to 1..=4096 above

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
