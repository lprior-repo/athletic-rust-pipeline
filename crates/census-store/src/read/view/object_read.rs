use fjall::Readable;
use sha2::{Digest, Sha256};

use super::object::JournalObject;
use super::object_wire::{
    alias_object, chunk_payload, index_ranges, manifest_bounds, JournalPath, RecordRange,
};
use super::StoreSnapshot;
use crate::journal::{
    expected_chunk_length, expected_index_entries, object_chunk_key, object_chunk_prefix,
    object_index_key, object_index_prefix, object_manifest_key, object_ordinal,
    OBJECT_INDEX_PAGE_ENTRIES,
};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

impl Store {
    pub fn journal_object(&self, phase: &str, key: &str) -> StoreResult<Option<JournalObject<'_>>> {
        self.snapshot().journal_object(phase, key)
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn journal_object(&self, phase: &str, key: &str) -> StoreResult<Option<JournalObject<'s>>> {
        let path = JournalPath { phase, key };
        let row_key = Store::journal_key(phase, key);
        let Some(raw) = read_row(&self.snapshot, self.journal, path, &row_key)? else {
            return Ok(None);
        };
        let object = alias_object(path, raw.as_ref())?;
        read_object_root(&self.snapshot, self.journal, path, &object, None).map(Some)
    }
}

pub(crate) fn read_context_object<'s>(
    snapshot: &fjall::Snapshot,
    journal: &'s fjall::Keyspace,
    root: &str,
) -> StoreResult<JournalObject<'s>> {
    read_context_object_bounded(
        snapshot,
        journal,
        root,
        ContextReadLimits {
            bytes: crate::journal::context::CONTEXT_TOTAL_BYTES,
            records: crate::journal::context::CONTEXT_RECORDS,
            record_bytes: crate::journal::context::CONTEXT_RECORD_BYTES,
        },
    )
}

#[derive(Clone, Copy)]
pub(crate) struct ContextReadLimits {
    pub(crate) bytes: usize,
    pub(crate) records: usize,
    pub(crate) record_bytes: usize,
}

pub(crate) fn read_context_object_bounded<'s>(
    snapshot: &fjall::Snapshot,
    journal: &'s fjall::Keyspace,
    root: &str,
    limits: ContextReadLimits,
) -> StoreResult<JournalObject<'s>> {
    let path = JournalPath {
        phase: "authority_context_v1",
        key: root,
    };
    read_object_root(snapshot, journal, path, root, Some(limits))
}

fn read_object_root<'s>(
    snapshot: &fjall::Snapshot,
    journal: &'s fjall::Keyspace,
    path: JournalPath<'_>,
    object: &str,
    budget: Option<ContextReadLimits>,
) -> StoreResult<JournalObject<'s>> {
    let manifest_key = object_manifest_key(object)?;
    let raw = read_row(snapshot, journal, path, &manifest_key)?.ok_or_else(|| {
        StoreError::JournalManifestMissing {
            phase: path.phase.to_string(),
            key: path.key.to_string(),
            object: object.to_owned(),
        }
    })?;
    let bounds = manifest_bounds(path, object, raw.as_ref())?;
    if let Some(limits) = budget {
        if bounds.total_bytes > limits.bytes || bounds.records > limits.records {
            return Err(path.refusal("authority context exceeds retained input limits"));
        }
    }
    let mut reader = JournalObject::new(
        path.phase.to_string(),
        path.key.to_string(),
        object.to_owned(),
        bounds,
        snapshot.clone(),
        journal,
    );
    if let Some(limits) = budget {
        reader.record_byte_limit = limits.record_bytes;
    }
    reader.verify()?;
    Ok(reader)
}

fn read_row(
    snapshot: &fjall::Snapshot,
    journal: &fjall::Keyspace,
    path: JournalPath<'_>,
    key: &[u8],
) -> StoreResult<Option<fjall::Slice>> {
    if let Some(length) = snapshot
        .size_of(journal, key)
        .map_err(|source| StoreError::Read { source })?
    {
        let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        super::legacy_wire::limit(path, "physical object row", bytes, MAX_JOURNAL_VALUE_BYTES)?;
    }
    snapshot
        .get(journal, key)
        .map_err(|source| StoreError::Read { source })
}

impl JournalObject<'_> {
    fn get(&self, key: &[u8]) -> StoreResult<Option<fjall::Slice>> {
        read_row(&self.snapshot, self.journal, self.path(), key)
    }

    fn verify(&self) -> StoreResult<()> {
        self.verify_keys(true)?;
        let mut next = 0usize;
        let mut records = 0usize;
        for page in 0..self.bounds.index_pages {
            for entry in self.read_page(page)? {
                next = self.validate_range(next, entry)?;
                records = records.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            }
        }
        if next != self.total_bytes() || records != self.record_count() {
            return Err(self
                .path()
                .refusal("index ranges do not cover the declared records"));
        }
        self.verify_keys(false)?;
        let mut hash = Sha256::new();
        for index in 0..self.chunk_count() {
            hash.update(self.read_chunk(index)?);
        }
        let computed = format!("{:x}", hash.finalize());
        if computed != self.object_id() {
            return Err(StoreError::JournalObjectDigest {
                phase: self.phase.clone(),
                key: self.key.clone(),
                declared: self.object_id().to_string(),
                computed,
            });
        }
        Ok(())
    }

    fn verify_keys(&self, index_pages: bool) -> StoreResult<()> {
        let prefix = if index_pages {
            object_index_prefix(self.object_id())?
        } else {
            object_chunk_prefix(self.object_id())?
        };
        for guard in self.snapshot.prefix(self.journal, &prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let ordinal = object_ordinal(raw.as_ref(), &prefix)
                .ok_or_else(|| self.path().refusal("object prefix holds a malformed key"))?;
            if index_pages && ordinal >= self.bounds.index_pages {
                return Err(StoreError::JournalIndexExtra {
                    phase: self.phase.clone(),
                    key: self.key.clone(),
                    page: ordinal,
                    declared: self.bounds.index_pages,
                });
            }
            if !index_pages && ordinal >= self.chunk_count() {
                return Err(StoreError::JournalChunkExtra {
                    phase: self.phase.clone(),
                    key: self.key.clone(),
                    index: ordinal,
                    declared: self.chunk_count(),
                });
            }
        }
        Ok(())
    }

    fn validate_range(&self, next: usize, entry: RecordRange) -> StoreResult<usize> {
        if entry.length == 0 || entry.length > self.record_byte_limit {
            return Err(StoreError::RowTooLarge {
                bytes: entry.length,
                max: self.record_byte_limit,
            });
        }
        let end = next
            .checked_add(entry.length)
            .ok_or(StoreError::CounterOverflow)?;
        if entry.offset != next || end > self.total_bytes() {
            return Err(self
                .path()
                .refusal("index ranges do not tile the declared bytes"));
        }
        Ok(end)
    }

    pub(super) fn read_page(&self, page: usize) -> StoreResult<Vec<RecordRange>> {
        let key = object_index_key(self.object_id(), page)?;
        let raw = self
            .get(&key)?
            .ok_or_else(|| StoreError::JournalIndexMissing {
                phase: self.phase.clone(),
                key: self.key.clone(),
                page,
            })?;
        let entries = expected_index_entries(page, self.bounds.index_pages, self.record_count())
            .ok_or_else(|| {
                self.path()
                    .refusal("index pages do not cover the declared records")
            })?;
        let first = page
            .checked_mul(OBJECT_INDEX_PAGE_ENTRIES)
            .ok_or(StoreError::CounterOverflow)?;
        index_ranges(
            self.path(),
            self.object_id(),
            page,
            first,
            entries,
            raw.as_ref(),
        )
    }

    pub(super) fn read_chunk(&self, index: usize) -> StoreResult<Vec<u8>> {
        let expected = expected_chunk_length(index, self.chunk_count(), self.total_bytes())
            .ok_or_else(|| {
                self.path()
                    .refusal("chunk count does not cover the declared bytes")
            })?;
        let digest = self.bounds.chunk_digests.get(index).ok_or_else(|| {
            self.path()
                .refusal("chunk digests do not cover the declared chunks")
        })?;
        let key = object_chunk_key(self.object_id(), index)?;
        let raw = self
            .get(&key)?
            .ok_or_else(|| StoreError::JournalChunkMissing {
                phase: self.phase.clone(),
                key: self.key.clone(),
                index,
            })?;
        chunk_payload(
            self.path(),
            self.object_id(),
            index,
            expected,
            digest,
            raw.as_ref(),
        )
    }
}

impl JournalObject<'_> {
    pub(crate) fn retained_bytes(&self, maximum: usize) -> StoreResult<Vec<u8>> {
        if self.total_bytes() > maximum {
            return Err(StoreError::RowTooLarge {
                bytes: self.total_bytes(),
                max: maximum,
            });
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.total_bytes())
            .map_err(|error| StoreError::Refused {
                detail: format!("retained object byte allocation: {error}"),
            })?;
        for index in 0..self.chunk_count() {
            bytes.extend_from_slice(&self.read_chunk(index)?);
        }
        if bytes.len() != self.total_bytes() {
            return Err(self.path().refusal("retained object byte length mismatch"));
        }
        Ok(bytes)
    }
}
