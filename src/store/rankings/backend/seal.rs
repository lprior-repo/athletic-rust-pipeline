//! Seal lifecycle: seal a collection to a snapshot digest and read the current seal.
use super::super::keys::seal_key;
use crate::domain::identity::EvidenceDigest;
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::Keyspace;

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

pub(super) fn load_seal(
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
