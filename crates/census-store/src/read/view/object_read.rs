use fjall::Readable;

use super::object::JournalObject;
use super::object_wire::{
    alias_object, chunk_payload, index_ranges, manifest_bounds, JournalPath, ObjectBounds,
    RecordRange,
};
use super::StoreSnapshot;
use crate::journal::{
    expected_chunk_length, expected_index_entries, object_chunk_key, object_chunk_prefix,
    object_index_key, object_index_prefix, object_manifest_key, object_ordinal,
    object_stream_digest, OBJECT_RECORD_BYTES_LIMIT,
};
use crate::{Store, StoreError, StoreResult};

impl Store {
    pub fn journal_object(&self, phase: &str, key: &str) -> StoreResult<Option<JournalObject>> {
        self.snapshot().journal_object(phase, key)
    }
}

impl StoreSnapshot<'_> {
    pub fn journal_object(&self, phase: &str, key: &str) -> StoreResult<Option<JournalObject>> {
        let row_key = Store::journal_key(phase, key);
        let Some(raw) = self.journal_get(&row_key)? else {
            return Ok(None);
        };
        let path = JournalPath { phase, key };
        let object = alias_object(path, raw.as_ref())?;
        let bounds = self.object_bounds(path, &object)?;
        let ranges = self.object_ranges(path, &object, &bounds)?;
        let chunks = self.object_chunks(path, &object, &bounds)?;
        let computed = object_stream_digest(&chunks);
        if computed != object {
            return Err(StoreError::JournalObjectDigest {
                phase: phase.to_string(),
                key: key.to_string(),
                declared: object,
                computed,
            });
        }
        Ok(Some(JournalObject::new(
            phase.to_string(),
            key.to_string(),
            object,
            bounds.total_bytes,
            ranges,
            chunks,
        )))
    }

    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<fjall::Slice>> {
        self.snapshot
            .get(self.journal, key)
            .map_err(|source| StoreError::Read { source })
    }

    fn object_bounds(&self, path: JournalPath<'_>, object: &str) -> StoreResult<ObjectBounds> {
        let key = object_manifest_key(object)?;
        let Some(raw) = self.journal_get(&key)? else {
            return Err(StoreError::JournalManifestMissing {
                phase: path.phase.to_string(),
                key: path.key.to_string(),
                object: object.to_string(),
            });
        };
        manifest_bounds(path, object, raw.as_ref())
    }

    fn object_ranges(
        &self,
        path: JournalPath<'_>,
        object: &str,
        bounds: &ObjectBounds,
    ) -> StoreResult<Vec<RecordRange>> {
        self.verify_index_pages(path, object, bounds)?;
        let mut ranges = Vec::new();
        ranges
            .try_reserve_exact(bounds.records)
            .map_err(|source| StoreError::Refused {
                detail: format!(
                    "journal index of {} records cannot be reserved: {source}",
                    bounds.records
                ),
            })?;
        let mut next = 0_usize;
        for page in 0..bounds.index_pages {
            let key = object_index_key(object, page)?;
            let Some(raw) = self.journal_get(&key)? else {
                return Err(StoreError::JournalIndexMissing {
                    phase: path.phase.to_string(),
                    key: path.key.to_string(),
                    page,
                });
            };
            let expected_entries = expected_index_entries(page, bounds.index_pages, bounds.records)
                .ok_or_else(|| path.refusal("index pages do not cover the declared records"))?;
            let entries = index_ranges(
                path,
                object,
                page,
                ranges.len(),
                expected_entries,
                raw.as_ref(),
            )?;
            for entry in entries {
                next = Self::validate_and_record_range(
                    path,
                    next,
                    entry,
                    &mut ranges,
                    bounds.total_bytes,
                )?;
            }
        }
        if next != bounds.total_bytes || ranges.len() != bounds.records {
            return Err(path.refusal("index ranges do not cover the declared records"));
        }
        Ok(ranges)
    }

    fn verify_index_pages(
        &self,
        path: JournalPath<'_>,
        object: &str,
        bounds: &ObjectBounds,
    ) -> StoreResult<()> {
        let prefix = object_index_prefix(object)?;
        for guard in self.snapshot.prefix(self.journal, &prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let page = object_ordinal(raw.as_ref(), &prefix)
                .ok_or_else(|| path.refusal("index prefix holds a malformed key"))?;
            if page >= bounds.index_pages {
                return Err(StoreError::JournalIndexExtra {
                    phase: path.phase.to_string(),
                    key: path.key.to_string(),
                    page,
                    declared: bounds.index_pages,
                });
            }
        }
        Ok(())
    }

    fn validate_and_record_range(
        path: JournalPath<'_>,
        next: usize,
        entry: RecordRange,
        ranges: &mut Vec<RecordRange>,
        total_bytes: usize,
    ) -> StoreResult<usize> {
        if entry.length == 0 || entry.length > OBJECT_RECORD_BYTES_LIMIT {
            return Err(StoreError::RowTooLarge {
                bytes: entry.length,
                max: OBJECT_RECORD_BYTES_LIMIT,
            });
        }
        let end = next
            .checked_add(entry.length)
            .ok_or(StoreError::CounterOverflow)?;
        if entry.offset != next || end > total_bytes {
            return Err(path.refusal("index ranges do not tile the declared bytes"));
        }
        ranges.push(entry);
        Ok(end)
    }

    fn object_chunks(
        &self,
        path: JournalPath<'_>,
        object: &str,
        bounds: &ObjectBounds,
    ) -> StoreResult<Vec<Vec<u8>>> {
        let prefix = object_chunk_prefix(object)?;
        for guard in self.snapshot.prefix(self.journal, &prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let index = object_ordinal(raw.as_ref(), &prefix)
                .ok_or_else(|| path.refusal("chunk prefix holds a malformed key"))?;
            if index >= bounds.chunks {
                return Err(StoreError::JournalChunkExtra {
                    phase: path.phase.to_string(),
                    key: path.key.to_string(),
                    index,
                    declared: bounds.chunks,
                });
            }
        }
        let mut chunks = Vec::new();
        chunks
            .try_reserve_exact(bounds.chunks)
            .map_err(|source| StoreError::Refused {
                detail: format!(
                    "journal object of {} chunks cannot be reserved: {source}",
                    bounds.chunks
                ),
            })?;
        for index in 0..bounds.chunks {
            let expected = expected_chunk_length(index, bounds.chunks, bounds.total_bytes)
                .ok_or_else(|| path.refusal("chunk count does not cover the declared bytes"))?;
            let digest = bounds
                .chunk_digests
                .get(index)
                .ok_or_else(|| path.refusal("chunk digests do not cover the declared chunks"))?;
            let key = object_chunk_key(object, index)?;
            let Some(raw) = self.journal_get(&key)? else {
                return Err(StoreError::JournalChunkMissing {
                    phase: path.phase.to_string(),
                    key: path.key.to_string(),
                    index,
                });
            };
            chunks.push(chunk_payload(
                path,
                object,
                index,
                expected,
                digest,
                raw.as_ref(),
            )?);
        }
        Ok(chunks)
    }
}
