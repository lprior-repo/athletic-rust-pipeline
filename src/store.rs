#![forbid(unsafe_code)]

mod audit;
mod backend;
mod error;
mod keys;
mod paging;
mod root;

pub use audit::AttemptEvidence;
pub use error::{Result, StoreError};

use crate::domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest};
use crate::model::SourceRecord;
use backend::StoreInner;
use std::{path::Path, sync::Arc};

/// Maximum size of one retained source document.
pub const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
/// Maximum number of source records in one atomic publication.
pub const MAX_BATCH_RECORDS: usize = 4_096;
/// Maximum serialized source-record bytes in one atomic publication.
pub const MAX_BATCH_BYTES: usize = 32 * 1024 * 1024;
/// Bounded Fjall block cache used by each opened artifact database.
pub const CACHE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone)]
pub struct ArtifactStore {
    inner: Arc<StoreInner>,
}

impl ArtifactStore {
    pub fn open(path: &Path) -> Result<Self> {
        StoreInner::open(path).map(|inner| Self {
            inner: Arc::new(inner),
        })
    }

    pub fn put_bytes(&self, bytes: &[u8]) -> Result<EvidenceDigest> {
        self.inner.put_bytes(bytes)
    }

    pub fn get_bytes(&self, digest: &EvidenceDigest) -> Result<Vec<u8>> {
        self.inner.get_bytes(digest)
    }

    pub fn put_source_batch(
        &self,
        workbook: &WorkbookDigest,
        records: &[SourceRecord],
    ) -> Result<()> {
        self.inner.put_source_batch(workbook, records)
    }

    pub fn source_record(
        &self,
        workbook: &WorkbookDigest,
        key: &SourceRowKey,
    ) -> Result<Option<SourceRecord>> {
        self.inner.source_record(workbook, key)
    }

    pub fn source_key_page(
        &self,
        workbook: &WorkbookDigest,
        sheet: &str,
        after_row: u32,
        limit: usize,
    ) -> Result<Vec<SourceRowKey>> {
        self.inner
            .source_key_page(workbook, sheet, after_row, limit)
    }

    pub fn visit_source<F>(&self, workbook: &WorkbookDigest, visitor: F) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        self.inner.visit_source(workbook, visitor)
    }
}

#[cfg(test)]
mod tests;
