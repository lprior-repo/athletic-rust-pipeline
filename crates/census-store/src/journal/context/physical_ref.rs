use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{bounded, refused, reserved, ContextCharge, ObjectPlan, CONTEXT_CHUNKS, CONTEXT_PAGES};
use crate::journal::{object_chunk_key, object_index_key, object_manifest_key};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

mod audit;
mod wire;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "wire::PhysicalRef")]
pub(crate) struct ContextPhysicalRef {
    root: String,
    full_bytes: u64,
    jsonl_bytes: u64,
    records: u64,
    manifest: RowFingerprint,
    chunks: Vec<RowFingerprint>,
    pages: Vec<RowFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RowFingerprint {
    length: u64,
    digest: [u8; 32],
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ContextPhysicalPlan {
    physical: ContextPhysicalRef,
    incremental: ContextCharge,
}

impl ContextPhysicalPlan {
    pub(super) fn new(physical: ContextPhysicalRef, incremental: ContextCharge) -> Self {
        Self {
            physical,
            incremental,
        }
    }

    pub(crate) fn root(&self) -> &str {
        self.physical.root()
    }

    pub(crate) fn full_bytes(&self) -> u64 {
        self.physical.full_bytes()
    }

    pub(crate) fn jsonl_bytes(&self) -> u64 {
        self.physical.jsonl_bytes()
    }

    pub(crate) fn records(&self) -> u64 {
        self.physical.records()
    }

    pub(crate) fn incremental_charge(&self) -> &ContextCharge {
        &self.incremental
    }

    pub(crate) fn physical_ref(&self) -> &ContextPhysicalRef {
        &self.physical
    }

    pub(crate) fn row_count(&self) -> usize {
        self.physical.row_count()
    }

    pub(crate) fn metadata_bytes_bound(&self) -> usize {
        self.physical.metadata_bytes_bound()
    }

    pub(crate) fn audit(&self, store: &Store) -> StoreResult<()> {
        if self.incremental.roots == 0 {
            self.physical.audit_ready(store)
        } else {
            self.physical.audit(store)
        }
    }
}

impl ContextPhysicalRef {
    pub(super) fn from_plan(
        plan: ObjectPlan,
        physical: PhysicalRows,
        full_bytes: u64,
    ) -> StoreResult<Self> {
        let result = Self {
            root: plan.staged.object,
            full_bytes,
            jsonl_bytes: u64::try_from(plan.staged.total_bytes)
                .map_err(|_| StoreError::CounterOverflow)?,
            records: u64::try_from(plan.staged.records).map_err(|_| StoreError::CounterOverflow)?,
            manifest: physical.manifest,
            chunks: physical.chunks,
            pages: physical.pages,
        };
        result.validate()?;
        Ok(result)
    }

    pub(crate) fn root(&self) -> &str {
        &self.root
    }

    pub(crate) fn full_bytes(&self) -> u64 {
        self.full_bytes
    }

    pub(crate) fn jsonl_bytes(&self) -> u64 {
        self.jsonl_bytes
    }

    pub(crate) fn records(&self) -> u64 {
        self.records
    }

    pub(crate) fn row_count(&self) -> usize {
        std::iter::once(&self.manifest)
            .chain(self.chunks.iter())
            .chain(self.pages.iter())
            .count()
    }

    pub(crate) fn metadata_bytes_bound(&self) -> usize {
        self.row_count().saturating_mul(256).saturating_add(512)
    }

    fn validate(&self) -> StoreResult<()> {
        let jsonl = usize::try_from(self.jsonl_bytes).map_err(|_| StoreError::CounterOverflow)?;
        let records = usize::try_from(self.records).map_err(|_| StoreError::CounterOverflow)?;
        bounded(
            "physical context JSONL bytes",
            jsonl,
            super::CONTEXT_TOTAL_BYTES,
        )?;
        bounded("physical context records", records, super::CONTEXT_RECORDS)?;
        if !crate::journal::is_object_id(&self.root)
            || self.chunks.len() != crate::journal::expected_chunk_count(jsonl)
            || self.pages.len() != crate::journal::expected_index_pages(records)
            || (records == 0) != (jsonl == 0)
            || records > jsonl
        {
            return Err(refused("invalid physical context inventory shape"));
        }
        let manifest = self
            .manifest
            .physical_bytes(&object_manifest_key(&self.root)?)?;
        let chunks = self
            .chunks
            .iter()
            .enumerate()
            .try_fold(manifest, |sum, (index, row)| {
                sum.checked_add(row.physical_bytes(&object_chunk_key(&self.root, index)?)?)
                    .ok_or(StoreError::CounterOverflow)
            })?;
        let total = self
            .pages
            .iter()
            .enumerate()
            .try_fold(chunks, |sum, (page, row)| {
                sum.checked_add(row.physical_bytes(&object_index_key(&self.root, page)?)?)
                    .ok_or(StoreError::CounterOverflow)
            })?;
        if total == 0 || total != self.full_bytes {
            return Err(refused("physical context inventory byte total mismatch"));
        }
        Ok(())
    }
}

impl RowFingerprint {
    fn new(raw: &[u8]) -> StoreResult<Self> {
        bounded(
            "context inventory row bytes",
            raw.len(),
            MAX_JOURNAL_VALUE_BYTES,
        )?;
        Ok(Self {
            length: u64::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)?,
            digest: Sha256::digest(raw).into(),
        })
    }

    fn physical_bytes(&self, key: &[u8]) -> StoreResult<u64> {
        let length = usize::try_from(self.length).map_err(|_| StoreError::CounterOverflow)?;
        bounded(
            "context inventory row bytes",
            length,
            MAX_JOURNAL_VALUE_BYTES,
        )?;
        if length == 0 {
            return Err(refused("empty physical context inventory row"));
        }
        self.length
            .checked_add(u64::try_from(key.len()).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)
    }
}

pub(super) struct PhysicalRows {
    manifest: RowFingerprint,
    chunks: Vec<RowFingerprint>,
    pages: Vec<RowFingerprint>,
}

impl PhysicalRows {
    pub(super) fn new(plan: &ObjectPlan, manifest: &[u8]) -> StoreResult<Self> {
        Ok(Self {
            manifest: RowFingerprint::new(manifest)?,
            chunks: reserved(plan.staged.chunks)?,
            pages: reserved(plan.page_digests.len())?,
        })
    }

    pub(super) fn chunk(&mut self, index: usize, raw: &[u8]) -> StoreResult<()> {
        Self::append(&mut self.chunks, index, CONTEXT_CHUNKS, raw)
    }

    pub(super) fn page(&mut self, index: usize, raw: &[u8]) -> StoreResult<()> {
        Self::append(&mut self.pages, index, CONTEXT_PAGES, raw)
    }

    fn append(
        rows: &mut Vec<RowFingerprint>,
        ordinal: usize,
        maximum: usize,
        raw: &[u8],
    ) -> StoreResult<()> {
        if ordinal != rows.len() || rows.len() >= maximum {
            return Err(refused("physical context inventory ordinal exceeded bound"));
        }
        rows.try_reserve(1)
            .map_err(|source| refused(&format!("context inventory allocation failed: {source}")))?;
        rows.push(RowFingerprint::new(raw)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
