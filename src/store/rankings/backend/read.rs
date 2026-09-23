//! Reference lookups: read the name and athlete reference indexes of a collection.
use super::super::keys::{
    athlete_ref_key, athlete_ref_prefix, collection_page_prefix, name_ref_key, name_ref_prefix,
};
use super::super::page_identity::{accepted_pages, AcceptedPages};
use super::super::types::{RankingLookup, RankingRecordRef};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::{Readable, Snapshot};

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
    let accepted = accepted_pages(&snap, store, &collection_page_prefix(collection))?;
    let prefix = name_ref_prefix(collection, name.as_str());
    collect_refs(&snap, store, &prefix, limit, &accepted, |key, entry| {
        // The key must name the record the value decodes to: a value rewritten
        // to another checkpoint, kind or record index is corruption, not a hit.
        let expected = name_ref_key(
            collection,
            name.as_str(),
            entry.athlete_id,
            &entry.checkpoint,
            entry.kind,
            entry.record_index,
        )?;
        (expected.as_slice() == key)
            .then_some(())
            .ok_or(StoreError::CorruptData)
    })
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
    let accepted = accepted_pages(&snap, store, &collection_page_prefix(collection))?;
    let prefix = athlete_ref_prefix(collection, athlete);
    collect_refs(&snap, store, &prefix, limit, &accepted, |key, entry| {
        let expected = athlete_ref_key(collection, entry)?;
        (expected.as_slice() == key)
            .then_some(())
            .ok_or(StoreError::CorruptData)
    })
}

/// Collect the reference records of one prefix that the collection accepts.
///
/// A stored reference that does not decode, or whose key does not name the
/// record the value encodes, is `CorruptData`: an absent record and a corrupt
/// record are different answers, and a skipped record would turn corruption into
/// "this candidate appears absent".
fn collect_refs(
    snap: &Snapshot,
    store: &StoreInner,
    prefix: &[u8],
    limit: usize,
    accepted: &AcceptedPages,
    key_matches: impl Fn(&[u8], &RankingRecordRef) -> Result<(), StoreError>,
) -> Result<RankingLookup, StoreError> {
    let need = limit.saturating_add(1); // limit is validated to 1..=4096 above
    let mut records = Vec::new();

    for guard in snap.prefix(&store.rankings, prefix) {
        let (key, value) = guard.into_inner().map_err(|_| StoreError::CorruptData)?;
        let ref_entry: RankingRecordRef =
            serde_json::from_slice(value.as_ref()).map_err(|_| StoreError::CorruptData)?;
        key_matches(key.as_ref(), &ref_entry)?;
        // Records of an abandoned capture stay stored but unaccepted: the page
        // marker that would make them live was dropped or republished.
        if !accepted.retains(ref_entry.checkpoint.as_str().as_bytes()) {
            continue;
        }
        records.push(ref_entry);
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
