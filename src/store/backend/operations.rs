//! The open store's operations: content-addressed documents and the source index.
//!
//! Opening the handle and validating the key layout stay in `backend.rs`; the
//! behaviour that runs against an open handle lives here, so the entry gate has
//! a boundary of its own and cannot be bypassed by an operation.
use super::StoreInner;
use crate::{
    domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest},
    model::SourceRecord,
    store::{
        error::{map_database_error, Result, StoreError},
        keys::{
            decode_source_key, digest_for, document_key, source_key, source_prefix,
            valid_document_size, verify_digest,
        },
        MAX_BATCH_BYTES, MAX_BATCH_RECORDS,
    },
};
use fjall::PersistMode;
use std::{collections::BTreeMap, str};

impl StoreInner {
    pub fn put_bytes(&self, bytes: &[u8]) -> Result<EvidenceDigest> {
        let _writer = self.writer.lock().map_err(|_| StoreError::Database)?;
        valid_document_size(bytes)?;
        let digest = digest_for(bytes)?;
        let key = document_key(&digest);
        match load_document(self, &key)? {
            Some(stored) => verify_digest(&digest, &stored)?,
            None => self.commit_document(key, bytes.to_vec())?,
        }
        Ok(digest)
    }

    pub fn get_bytes(&self, digest: &EvidenceDigest) -> Result<Vec<u8>> {
        let key = document_key(digest);
        let stored = load_document(self, &key)?.ok_or(StoreError::MissingArtifact)?;
        verify_digest(digest, &stored).map(|()| stored)
    }

    pub fn put_source_batch(
        &self,
        workbook: &WorkbookDigest,
        records: &[SourceRecord],
    ) -> Result<()> {
        let _writer = self.writer.lock().map_err(|_| StoreError::Database)?;
        if records.len() > MAX_BATCH_RECORDS {
            return Err(StoreError::BatchTooLarge);
        }
        let mut plan = BatchPlan::default();
        records
            .iter()
            .try_for_each(|record| self.plan_record(workbook, record, &mut plan))?;
        plan.commit(self)
    }

    pub fn source_record(
        &self,
        workbook: &WorkbookDigest,
        key: &SourceRowKey,
    ) -> Result<Option<SourceRecord>> {
        let index_key = source_key(workbook, key)?;
        let Some(index_value) = self.sources.get(&index_key).map_err(map_database_error)? else {
            return Ok(None);
        };
        let digest = parse_index_digest(index_value.as_ref())?;
        let record = self.read_record(&digest)?;
        validate_stored_record(&record, key)?;
        Ok(Some(record))
    }

    pub fn visit_source<F>(&self, workbook: &WorkbookDigest, mut visitor: F) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        let prefix = source_prefix(workbook);
        self.sources
            .prefix(&prefix)
            .try_for_each(|guard| self.visit_guard(guard, &mut visitor))
    }

    fn plan_record(
        &self,
        workbook: &WorkbookDigest,
        record: &SourceRecord,
        plan: &mut BatchPlan,
    ) -> Result<()> {
        let row = validate_input_record(record)?;
        let bytes = serde_json::to_vec(record)?;
        valid_document_size(&bytes)?;
        plan.add_bytes(bytes.len())?;
        let digest = digest_for(&bytes)?;
        let doc_key = document_key(&digest);
        ensure_document(self, &doc_key, &digest, &bytes, plan)?;
        let index_key = source_key(workbook, &row)?;
        self.plan_index(index_key, digest, plan)
    }

    fn plan_index(
        &self,
        index_key: Vec<u8>,
        digest: EvidenceDigest,
        plan: &mut BatchPlan,
    ) -> Result<()> {
        let encoded = digest.as_str().as_bytes().to_vec();
        let existing = self.sources.get(&index_key).map_err(map_database_error)?;
        if let Some(value) = existing {
            let current = parse_index_digest(value.as_ref())?;
            return (current == digest)
                .then_some(())
                .ok_or(StoreError::SourceConflict);
        }
        if let Some(current) = plan.indexes.get(&index_key) {
            return (current == &encoded)
                .then_some(())
                .ok_or(StoreError::SourceConflict);
        }
        plan.indexes.insert(index_key, encoded);
        Ok(())
    }

    fn commit_document(&self, key: Vec<u8>, bytes: Vec<u8>) -> Result<()> {
        let mut batch = self.database.batch();
        batch.insert(&self.documents, key, bytes);
        batch
            .durability(Some(PersistMode::SyncAll))
            .commit()
            .map_err(map_database_error)
    }

    fn visit_guard<F>(&self, guard: fjall::Guard, visitor: &mut F) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        let (key, value) = guard.into_inner().map_err(|_| StoreError::CorruptData)?;
        let (sheet, row) = decode_source_key(key.as_ref()).ok_or(StoreError::CorruptData)?;
        let digest = parse_index_digest(value.as_ref())?;
        let record = self.read_record(&digest)?;
        validate_stored_key(&record, &sheet, row)?;
        visitor(record)
    }

    fn read_record(&self, digest: &EvidenceDigest) -> Result<SourceRecord> {
        self.get_bytes(digest)
            .map_err(|error| match error {
                StoreError::MissingArtifact => StoreError::CorruptData,
                other => other,
            })
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(StoreError::from))
    }
}

#[derive(Default)]
struct BatchPlan {
    documents: BTreeMap<Vec<u8>, Vec<u8>>,
    indexes: BTreeMap<Vec<u8>, Vec<u8>>,
    bytes: usize,
}

impl BatchPlan {
    fn add_bytes(&mut self, count: usize) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(count)
            .ok_or(StoreError::BatchTooLarge)?;
        (self.bytes <= MAX_BATCH_BYTES)
            .then_some(())
            .ok_or(StoreError::BatchTooLarge)
    }

    fn commit(self, store: &StoreInner) -> Result<()> {
        if self.documents.is_empty() && self.indexes.is_empty() {
            return Ok(());
        }
        let mut batch = store.database.batch();
        self.documents
            .into_iter()
            .for_each(|(key, bytes)| batch.insert(&store.documents, key, bytes));
        self.indexes
            .into_iter()
            .for_each(|(key, digest)| batch.insert(&store.sources, key, digest));
        batch
            .durability(Some(PersistMode::SyncAll))
            .commit()
            .map_err(map_database_error)
    }
}

fn ensure_document(
    store: &StoreInner,
    key: &[u8],
    digest: &EvidenceDigest,
    bytes: &[u8],
    plan: &mut BatchPlan,
) -> Result<()> {
    match load_document(store, key)? {
        Some(stored) => verify_digest(digest, &stored),
        None => match plan.documents.get(key) {
            Some(stored) => verify_digest(digest, stored),
            None => {
                plan.documents.insert(key.to_vec(), bytes.to_vec());
                Ok(())
            }
        },
    }
}

fn load_document(store: &StoreInner, key: &[u8]) -> Result<Option<Vec<u8>>> {
    match store.documents.get(key).map_err(map_database_error)? {
        Some(value) => {
            let slice = value.as_ref();
            let size = slice.len();
            if size > crate::store::MAX_DOCUMENT_BYTES {
                return Err(StoreError::DocumentTooLarge);
            }
            Ok(Some(slice.to_vec()))
        }
        None => Ok(None),
    }
}

fn validate_input_record(record: &SourceRecord) -> Result<SourceRowKey> {
    let key = SourceRowKey::parse(&record.source_key).map_err(|_| StoreError::InvalidSourceRow)?;
    if key.sheet() != record.sheet || key.row() != record.excel_row {
        return Err(StoreError::InvalidSourceRow);
    }
    Ok(key)
}

fn validate_stored_record(record: &SourceRecord, requested: &SourceRowKey) -> Result<()> {
    validate_stored_key(record, requested.sheet(), requested.row())?;
    let stored = SourceRowKey::parse(&record.source_key).map_err(|_| StoreError::CorruptData)?;
    (stored.as_str() == requested.as_str())
        .then_some(())
        .ok_or(StoreError::CorruptData)
}

fn validate_stored_key(record: &SourceRecord, sheet: &str, row: u32) -> Result<()> {
    let stored = SourceRowKey::parse(&record.source_key).map_err(|_| StoreError::CorruptData)?;
    (record.sheet == sheet
        && record.excel_row == row
        && stored.sheet() == sheet
        && stored.row() == row)
        .then_some(())
        .ok_or(StoreError::CorruptData)
}

fn parse_index_digest(bytes: &[u8]) -> Result<EvidenceDigest> {
    let text = str::from_utf8(bytes).map_err(|_| StoreError::CorruptData)?;
    EvidenceDigest::parse(text).map_err(|_| StoreError::CorruptData)
}
