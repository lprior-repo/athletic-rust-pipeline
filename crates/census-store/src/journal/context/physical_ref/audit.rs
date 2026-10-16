use fjall::Readable;
use sha2::{Digest, Sha256};

use super::{ContextPhysicalRef, RowFingerprint};
use crate::journal::context::refused;
use crate::journal::{
    object_chunk_key, object_chunk_prefix, object_index_key, object_index_prefix,
    object_manifest_key, object_ordinal,
};
use crate::{Store, StoreError, StoreResult};

impl ContextPhysicalRef {
    pub(crate) fn audit(&self, store: &Store) -> StoreResult<()> {
        self.audit_presence(store, RequiredPresence::PartialAllowed)
    }

    pub(crate) fn audit_ready(&self, store: &Store) -> StoreResult<()> {
        self.audit_presence(store, RequiredPresence::Ready)
    }

    fn audit_presence(&self, store: &Store, required: RequiredPresence) -> StoreResult<()> {
        let snapshot = store.db.snapshot();
        let manifest = object_manifest_key(&self.root)?;
        let ready = self.manifest.verify(store, &snapshot, &manifest)?;
        if matches!(required, RequiredPresence::Ready) && !ready {
            return Err(refused("admitted ready physical context lost its manifest"));
        }
        verify_manifest_namespace(store, &snapshot, &manifest)?;
        verify_namespace(
            store,
            &snapshot,
            &object_chunk_prefix(&self.root)?,
            self.chunks.len(),
        )?;
        verify_namespace(
            store,
            &snapshot,
            &object_index_prefix(&self.root)?,
            self.pages.len(),
        )?;
        self.chunks
            .iter()
            .enumerate()
            .try_for_each(|(index, row)| {
                let present =
                    row.verify(store, &snapshot, &object_chunk_key(&self.root, index)?)?;
                require_complete(ready, present)
            })?;
        self.pages.iter().enumerate().try_for_each(|(page, row)| {
            let present = row.verify(store, &snapshot, &object_index_key(&self.root, page)?)?;
            require_complete(ready, present)
        })
    }
}

enum RequiredPresence {
    PartialAllowed,
    Ready,
}

impl RowFingerprint {
    fn verify(&self, store: &Store, snapshot: &fjall::Snapshot, key: &[u8]) -> StoreResult<bool> {
        let Some(length) = snapshot
            .size_of(&store.journal, key)
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(false);
        };
        let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        let expected = usize::try_from(self.length).map_err(|_| StoreError::CounterOverflow)?;
        if length != expected {
            return Err(refused("physical context row length mismatch"));
        }
        let raw = snapshot
            .get(&store.journal, key)
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| refused("physical context row disappeared from snapshot"))?;
        let digest: [u8; 32] = Sha256::digest(&raw).into();
        if raw.len() != expected || digest != self.digest {
            return Err(refused("physical context row fingerprint mismatch"));
        }
        Ok(true)
    }
}

fn require_complete(ready: bool, present: bool) -> StoreResult<()> {
    if ready && !present {
        return Err(refused("ready physical context has missing expected row"));
    }
    Ok(())
}

fn verify_manifest_namespace(
    store: &Store,
    snapshot: &fjall::Snapshot,
    manifest: &[u8],
) -> StoreResult<()> {
    snapshot
        .prefix(&store.journal, manifest)
        .take(2)
        .try_for_each(|guard| {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if key.as_ref() != manifest {
                return Err(refused("extra or malformed physical manifest row"));
            }
            Ok(())
        })
}

fn verify_namespace(
    store: &Store,
    snapshot: &fjall::Snapshot,
    prefix: &[u8],
    count: usize,
) -> StoreResult<()> {
    let length = prefix
        .len()
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    let scope = prefix
        .get(..length)
        .ok_or_else(|| refused("physical row prefix is malformed"))?;
    let maximum = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    snapshot
        .prefix(&store.journal, scope)
        .take(maximum)
        .try_for_each(|guard| {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            match object_ordinal(&key, prefix) {
                Some(ordinal) if ordinal < count => Ok(()),
                _ => Err(refused("extra or malformed physical context row")),
            }
        })
}
