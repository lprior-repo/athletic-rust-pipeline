use fjall::{PersistMode, Readable};

use super::{
    bounded, refused, reserved, wire, IndexEntry, ObjectPlan, CONTEXT_CHUNKS, CONTEXT_PAGES,
};
use crate::journal::{
    expected_chunk_length, object_chunk_key, object_chunk_prefix, object_digest, object_index_key,
    object_index_prefix, object_manifest_key, object_ordinal,
};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

#[derive(Clone, Copy)]
pub(super) enum ObjectPresence {
    Staging,
    Ready,
}

pub(super) struct StageTarget<'a> {
    store: &'a Store,
    plan: &'a ObjectPlan,
    presence: ObjectPresence,
    physical: &'a mut PhysicalDigests,
}

pub(super) struct PhysicalDigests {
    chunks: Vec<String>,
    pages: Vec<String>,
}

impl PhysicalDigests {
    pub(super) fn new() -> StoreResult<Self> {
        Ok(Self {
            chunks: reserved(CONTEXT_CHUNKS)?,
            pages: reserved(CONTEXT_PAGES)?,
        })
    }
}

impl<'a> StageTarget<'a> {
    pub(super) fn presence(&self) -> ObjectPresence {
        self.presence
    }

    pub(super) fn open(
        store: &'a Store,
        plan: &'a ObjectPlan,
        physical: &'a mut PhysicalDigests,
    ) -> StoreResult<Self> {
        let _appends = store.lock_appends();
        let key = object_manifest_key(&plan.staged.object)?;
        let expected = wire::manifest(plan)?;
        let presence = match read_row(store, &key)? {
            Some(existing) => {
                require_equal(&key, &existing, &expected)?;
                ObjectPresence::Ready
            }
            None => ObjectPresence::Staging,
        };
        verify_namespace(store, plan)?;
        Ok(Self {
            store,
            plan,
            presence,
            physical,
        })
    }

    pub(super) fn stage_chunk(
        &mut self,
        index: usize,
        bytes: &[u8],
        digest: &str,
    ) -> StoreResult<()> {
        if self.plan.chunk_digests.get(index).map(String::as_str) != Some(digest)
            || expected_chunk_length(index, self.plan.staged.chunks, self.plan.staged.total_bytes)
                != Some(bytes.len())
        {
            return Err(refused(&format!(
                "callback changed chunk {index} between passes"
            )));
        }
        let object = &self.plan.staged.object;
        let key = object_chunk_key(object, index)?;
        let value = wire::chunk(object, index, bytes, digest)?;
        persist_row(self.store, &key, &value, self.presence)?;
        self.physical.chunks.push(object_digest(&value));
        Ok(())
    }

    pub(super) fn stage_page(
        &mut self,
        page: usize,
        entries: &[IndexEntry],
        digest: &str,
    ) -> StoreResult<()> {
        if self.plan.page_digests.get(page).map(String::as_str) != Some(digest) {
            return Err(refused(&format!(
                "callback changed index page {page} between passes"
            )));
        }
        let object = &self.plan.staged.object;
        let key = object_index_key(object, page)?;
        let value = wire::page(object, page, entries)?;
        persist_row(self.store, &key, &value, self.presence)?;
        self.physical.pages.push(object_digest(&value));
        Ok(())
    }
}

pub(super) fn publish_manifest(
    store: &Store,
    plan: &ObjectPlan,
    physical: &PhysicalDigests,
    presence: ObjectPresence,
) -> StoreResult<()> {
    let _appends = store.lock_appends();
    verify_namespace(store, plan)?;
    verify_physical(store, plan, physical)?;
    let key = object_manifest_key(&plan.staged.object)?;
    let value = wire::manifest(plan)?;
    persist_row_locked(store, &key, &value, presence)
}

fn persist_row(
    store: &Store,
    key: &[u8],
    value: &[u8],
    presence: ObjectPresence,
) -> StoreResult<()> {
    let _appends = store.lock_appends();
    persist_row_locked(store, key, value, presence)
}

fn persist_row_locked(
    store: &Store,
    key: &[u8],
    value: &[u8],
    presence: ObjectPresence,
) -> StoreResult<()> {
    bounded(
        "context physical row bytes",
        value.len(),
        MAX_JOURNAL_VALUE_BYTES,
    )?;
    if let Some(existing) = read_row(store, key)? {
        return require_equal(key, &existing, value);
    }
    if matches!(presence, ObjectPresence::Ready) {
        return Err(refused(&format!(
            "ready object has missing physical row {key:?}"
        )));
    }
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, value);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn require_equal(key: &[u8], existing: &[u8], expected: &[u8]) -> StoreResult<()> {
    if existing != expected {
        return Err(refused(&format!(
            "immutable physical row conflicts at {key:?}"
        )));
    }
    Ok(())
}

fn read_row(store: &Store, key: &[u8]) -> StoreResult<Option<fjall::Slice>> {
    let snapshot = store.db.snapshot();
    if let Some(length) = snapshot
        .size_of(&store.journal, key)
        .map_err(|source| StoreError::Read { source })?
    {
        let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        bounded("context physical row bytes", bytes, MAX_JOURNAL_VALUE_BYTES)?;
    }
    snapshot
        .get(&store.journal, key)
        .map_err(|source| StoreError::Read { source })
}

fn verify_namespace(store: &Store, plan: &ObjectPlan) -> StoreResult<()> {
    let object = &plan.staged.object;
    [
        (object_chunk_prefix(object)?, plan.staged.chunks),
        (object_index_prefix(object)?, plan.page_digests.len()),
    ]
    .into_iter()
    .try_for_each(|(prefix, count)| verify_prefix(store, &prefix, count))
}

fn verify_prefix(store: &Store, prefix: &[u8], count: usize) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    let scan_bound = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    snapshot
        .prefix(&store.journal, prefix)
        .take(scan_bound)
        .try_for_each(|guard| {
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            match object_ordinal(&key, prefix) {
                Some(ordinal) if ordinal < count => Ok(()),
                _ => Err(refused(&format!("extra or malformed physical row {key:?}"))),
            }
        })
}

fn verify_physical(
    store: &Store,
    plan: &ObjectPlan,
    physical: &PhysicalDigests,
) -> StoreResult<()> {
    if physical.chunks.len() != plan.staged.chunks
        || physical.pages.len() != plan.page_digests.len()
    {
        return Err(refused(
            "staged row digests do not cover the complete context",
        ));
    }
    let object = &plan.staged.object;
    physical
        .chunks
        .iter()
        .enumerate()
        .try_for_each(|(index, digest)| {
            verify_row_digest(store, &object_chunk_key(object, index)?, digest)
        })?;
    physical
        .pages
        .iter()
        .enumerate()
        .try_for_each(|(page, digest)| {
            verify_row_digest(store, &object_index_key(object, page)?, digest)
        })
}

fn verify_row_digest(store: &Store, key: &[u8], digest: &str) -> StoreResult<()> {
    let existing = read_row(store, key)?
        .ok_or_else(|| refused(&format!("staged physical row disappeared at {key:?}")))?;
    if object_digest(&existing) != digest {
        return Err(refused(&format!("staged physical row changed at {key:?}")));
    }
    Ok(())
}
